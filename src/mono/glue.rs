//! *Glue*: the functions monomorphisation writes for what the program
//! doesn't spell out, as MIR, so that what runs is all MIR (B1 of the
//! backend design, docs/32.1-drops.md).
//!
//! - `{drop}::<T>(x: T)` drops a value of type `T`: by `T`'s own
//!   `discard`, if it has one; or else each of its parts that needs
//!   dropping, in order: a struct's fields, a closure's captures, an
//!   array's values, the fields of the variant an enum is, and what a box
//!   points to, before the box is freed. An `Arena<T>`'s values aren't
//!   known until it runs, so dropping one is the interpreter's (and a
//!   backend's runtime's), each value by `{drop}::<Box<T>>`.
//! - `{eq}::<T>(a: &T, b: &T) -> bool` is `==` on what isn't a number, a
//!   `bool`, a character or `()`: by `T`'s own `eq`, if it has one; or
//!   else field by field, stopping at the first that differs, through
//!   references and boxes to what they point to, and value by value for
//!   arrays, slices and strings, which a loop does.
//!
//! - `{debug}::<T, W>(x: &T, w: &mut W) -> Result<(), W::Error>` is the
//!   `fmt` of a derived `Debug`: the type's name, then its fields, each
//!   by its type's `Debug::fmt`, as `Point { x: 1, y: 2 }`, or the name of
//!   the variant the value is, then its fields, as `Some(3)`. Each write
//!   gives a `Result`, and the first `Err` is what it gives (lemma G1 of
//!   docs/43.1-formatting.md).
//!
//! - `{cmp}::<T>(a: &T, b: &T) -> Ordering` is the `cmp` of a derived
//!   `Ord`: field by field, each by its type's `Ord::cmp`, the first that
//!   isn't `Equal` deciding; of an enum, by the variants' order first,
//!   then the fields of the one both are. A tuple's is its fields' (lemma
//!   T4 of docs/58-tuples.md).
//!
//! - `{fn}::<C>(..) -> ..` is closure type `C`, which captures nothing,
//!   used as a function (*reified*, see `Checker::reify`): it makes the
//!   closure, which has nothing in it, and calls its body with it, by `&`.
//!
//! Each does what the interpreter did when it worked these out as it ran,
//! in the same order (E4).

use super::{Cx, Target};
use crate::ast::BinOp;
use crate::error::Span;
use crate::mir::*;
use crate::traits::{DEBUG, DISCARD, EQ, FnKind, ORD, Source, WRITE};
use crate::types::{Region, Type};

/// The name of glue `kind`, `{drop}` or `{eq}`, for type `t`, which has no
/// type parameters left in it.
pub(crate) fn glue_name(kind: &str, t: &Type) -> String {
    format!("{kind}::<{}>", t.erased().symbols())
}

pub(crate) const DROP: &str = "{drop}";
pub(crate) const EQ_GLUE: &str = "{eq}";
pub(crate) const FN_GLUE: &str = "{fn}";
pub(crate) const DEBUG_GLUE: &str = "{debug}";
pub(crate) const CMP_GLUE: &str = "{cmp}";

/// Is glue `name` one of these, which have bodies but no declarations?
pub(crate) fn is_glue(name: &str) -> bool {
    ["{drop}::<", "{eq}::<", "{fn}::<", "{debug}::<", "{cmp}::<"]
        .iter()
        .any(|g| name.starts_with(g))
}

/// A body being written.
struct G {
    locals: Vec<LocalDecl>,
    blocks: Vec<Block>,
    cur: usize,
    params: usize,
}

impl G {
    fn new(ret: Type, params: Vec<Type>) -> G {
        let n = params.len();
        let mut g = G {
            locals: Vec::new(),
            blocks: vec![Block::default()],
            cur: 0,
            params: n,
        };
        g.local(ret, LocalKind::Return);
        for p in params {
            g.local(p, LocalKind::Param);
        }
        g
    }

    fn local(&mut self, ty: Type, kind: LocalKind) -> Local {
        self.locals.push(LocalDecl {
            ty,
            kind,
            mutable: true,
            name: None,
            span: Span::new(0, 0),
            mut_span: None,
            by_ref: None,
            deferred: false,
            of_static: None,
        });
        Local(self.locals.len() - 1)
    }

    fn temp(&mut self, ty: Type) -> Local {
        self.local(ty, LocalKind::Temp)
    }

    fn push(&mut self, kind: StmtKind) {
        let s = Statement {
            kind,
            span: Span::new(0, 0),
        };
        self.blocks[self.cur].stmts.push(s);
    }

    fn assign(&mut self, place: Place, value: Rvalue) {
        self.push(StmtKind::Assign {
            place,
            value,
            declare: false,
        });
    }

    fn new_block(&mut self) -> usize {
        self.blocks.push(Block::default());
        self.blocks.len() - 1
    }

    fn terminate(&mut self, t: Term) {
        self.blocks[self.cur].term = Some(t);
    }

    /// Ends the body: its locals go out of scope, the last made first, and
    /// it returns. Its temporaries are `bool`s, numbers and references,
    /// which need no dropping.
    fn finish(mut self) -> Body {
        for l in (1..self.locals.len()).rev() {
            self.push(StmtKind::Dead(Local(l)));
        }
        self.terminate(Term::Return);
        Body {
            locals: self.locals,
            blocks: self.blocks,
            params: self.params,
            env: Vec::new(),
            changed_by_closures: Vec::new(),
            unique_by_closures: Vec::new(),
            heads: Vec::new(),
            drop_uses: Default::default(),
            refinements: Vec::new(),
            unsafe_blocks: Vec::new(),
        }
    }
}

fn bool_const(b: bool) -> Rvalue {
    Rvalue::Use(Operand::Const(Const::Bool(b)))
}

fn shared(t: &Type) -> Type {
    Type::Ref(Region::Erased, false, Box::new(t.clone()))
}

/// Is `t` compared by the interpreter, or a machine, directly: `==` on
/// it calls nothing?
pub(crate) fn scalar(t: &Type) -> bool {
    matches!(
        t,
        Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never
    )
}

impl Cx<'_, '_, '_> {
    /// Does dropping a value of type `t`, with no type parameters left in
    /// it, do anything: free memory, call a `discard`, or throw away what
    /// isn't `Discard` (which the interpreter reports as a leak)?
    pub(super) fn needs_drop(&mut self, t: &Type) -> bool {
        let t = t.erased();
        if let Some(&known) = self.m.needs_drop.get(&t) {
            return known;
        }
        // A type that holds itself does so through a box, which needs
        // dropping anyway: so the guess never decides.
        self.m.needs_drop.insert(t.clone(), true);
        let needs = match &t {
            Type::Box(_) => true,
            Type::Array(elem, n) => **n != Type::Nat(0) && self.needs_drop(elem),
            Type::Tuple(ts) => ts.iter().any(|t| self.needs_drop(t)),
            Type::Adt(symbol, ..) if crate::types::is_closure(symbol) => {
                let fields = self.m.info.traits.closures[symbol].fields_at(&t);
                fields.iter().any(|f| self.needs_drop(f))
            }
            Type::Adt(symbol, ..) if symbol == crate::check::ARENA_SLOTS => true,
            Type::Adt(symbol, ..) => {
                let traits = &self.m.info.traits;
                match traits.implements(&t, DISCARD, &[], &[]) {
                    Ok(Source::Impl(i, _)) if !traits.impls[i].derived => true,
                    Ok(_) if crate::intrinsic::is_intrinsic_type(symbol) => false,
                    Ok(_) => {
                        let fields = self.field_types(&t);
                        fields.iter().any(|f| self.needs_drop(f))
                    }
                    Err(_) => true,
                }
            }
            _ => false,
        };
        self.m.needs_drop.insert(t, needs);
        needs
    }

    /// The name of the drop glue of `t`, made if it isn't yet.
    pub(super) fn drop_glue(&mut self, t: &Type) -> String {
        let t = t.erased();
        let name = glue_name(DROP, &t);
        if !self.m.glue_made.insert(name.clone()) {
            return name;
        }
        if let Type::Adt(symbol, _, args) = &t
            && symbol == crate::check::ARENA_SLOTS
        {
            // The interpreter's: each value by the glue of its box.
            self.drop_glue(&Type::Box(Box::new(args[0].clone())));
            return name;
        }
        let body = self.drop_body(&t);
        self.m.bodies.insert(name.clone(), body);
        name
    }

    fn drop_body(&mut self, t: &Type) -> Body {
        let mut g = G::new(Type::Unit, vec![t.clone()]);
        let x = Place::local(Local(1));
        let traits = &self.m.info.traits;
        let own = matches!(
            traits.implements(t, DISCARD, &[], &[]),
            Ok(Source::Impl(i, _)) if !traits.impls[i].derived
        );
        if own {
            let name = format!("<{DISCARD}>::discard");
            let Target::Fn(of, args, copy) = self.target(&name, vec![t.clone()]) else {
                unreachable!("an own `discard` is a function")
            };
            let f = self.m.named_copy(&of, args, copy);
            let call = Rvalue::Call(
                Operand::Const(Const::Fn(f, Vec::new())),
                vec![Operand::Move(x)],
            );
            g.assign(Place::local(RETURN), call);
            return g.finish();
        }
        g.assign(
            Place::local(RETURN),
            Rvalue::Use(Operand::Const(Const::Unit)),
        );
        // As a function value, `Discard::discard` of what needs nothing.
        if !self.needs_drop(t) {
            return g.finish();
        }
        match t {
            Type::Box(inner) => {
                self.drop_part(&mut g, x.project(Proj::Deref), inner);
                g.push(StmtKind::Free(x));
            }
            Type::Array(elem, n) => {
                let Type::Nat(n) = **n else {
                    unreachable!("an array of no length")
                };
                for i in 0..n as usize {
                    self.drop_part(&mut g, x.project(Proj::Field(i)), elem);
                }
            }
            // As a struct's fields, in order (lemma T1 of docs/58-tuples.md).
            Type::Tuple(ts) => {
                for (i, f) in ts.iter().enumerate() {
                    self.drop_part(&mut g, x.project(Proj::Field(i)), f);
                }
            }
            // Not `Discard`: the interpreter stops at the leak first.
            Type::Adt(symbol, ..) if crate::intrinsic::is_intrinsic_type(symbol) => {
                g.terminate(Term::Unreachable);
                g.cur = g.new_block();
            }
            Type::Adt(symbol, ..) if crate::types::is_closure(symbol) => {
                let fields = self.m.info.traits.closures[symbol].fields_at(t);
                for (i, f) in fields.iter().enumerate() {
                    self.drop_part(&mut g, x.project(Proj::Field(i)), f);
                }
            }
            Type::Adt(..) => match self.variant_types(t) {
                None => {
                    for (i, f) in self.field_types(t).iter().enumerate() {
                        self.drop_part(&mut g, x.project(Proj::Field(i)), f);
                    }
                }
                Some(variants) => {
                    let join = g.new_block();
                    let start = g.cur;
                    let mut pats = Vec::new();
                    let mut targets = Vec::new();
                    for (v, fields) in variants.iter().enumerate() {
                        g.cur = g.new_block();
                        targets.push(Bb(g.cur));
                        pats.push(Pat::Variant(v, vec![Pat::Any; fields.len()]));
                        for (i, f) in fields.iter().enumerate() {
                            self.drop_part(&mut g, x.project(Proj::Variant(v, i)), f);
                        }
                        g.terminate(Term::Goto(Bb(join)));
                    }
                    g.cur = start;
                    g.terminate(Term::Match(x.clone(), pats, targets));
                    g.cur = join;
                }
            },
            t => unreachable!("drop glue of `{t}`"),
        }
        g.finish()
    }

    fn drop_part(&mut self, g: &mut G, place: Place, t: &Type) {
        if self.needs_drop(t) {
            self.drop_glue(t);
            g.push(StmtKind::Drop(place, t.erased()));
        }
    }

    /// The types of the fields of each variant of `t`, if it's an enum.
    pub(super) fn variant_types(&mut self, t: &Type) -> Option<Vec<Vec<Type>>> {
        let Type::Adt(name, _, args) = t else {
            return None;
        };
        let d = *self.m.types.get(name.as_str())?;
        let crate::ast::TypeDecl::Enum(e) = d else {
            return None;
        };
        let params = super::names(d.generics());
        let s = crate::types::Subst::new(&params, args);
        let cx = Cx {
            m: &mut *self.m,
            s: &s,
            params,
        };
        Some(
            (e.variants.iter())
                .map(|v| {
                    v.fields
                        .iter()
                        .map(|f| cx.resolved(&super::unrefined(f)))
                        .collect()
                })
                .collect(),
        )
    }

    /// What compares two values of type `t`, `==`, if a function does: by
    /// name, `t`'s own `eq`, or else its glue. `None` for what's compared
    /// directly.
    pub(super) fn eq_fn(&mut self, t: &Type) -> Option<String> {
        let t = t.erased();
        if scalar(&t) {
            return None;
        }
        Some(match self.target(&format!("<{EQ}>::eq"), vec![t.clone()]) {
            Target::Fn(of, args, copy) => self.m.named_copy(&of, args, copy),
            Target::Eq(t) => self.eq_glue(&t),
            Target::Discard(_) | Target::Debug(..) | Target::Cmp(_) => {
                unreachable!("`eq` of `Eq`")
            }
        })
    }

    /// The name of the `==` glue of `t`, made if it isn't yet.
    pub(super) fn eq_glue(&mut self, t: &Type) -> String {
        let t = t.erased();
        let name = glue_name(EQ_GLUE, &t);
        if self.m.glue_made.insert(name.clone()) {
            let body = self.eq_body(&t);
            self.m.bodies.insert(name.clone(), body);
        }
        name
    }

    /// `a == b`, of the values at `a` and `b`, of type `t`, into a new
    /// `bool` temporary.
    fn compare(&mut self, g: &mut G, t: &Type, a: Place, b: Place) -> Local {
        let r = g.temp(Type::Bool);
        match self.eq_fn(t) {
            None => {
                let id = self.m.id();
                let value = Rvalue::Binary(BinOp::Eq, Operand::Copy(a), Operand::Copy(b), id);
                g.assign(Place::local(r), value);
            }
            Some(f) => {
                let (ra, rb) = (g.temp(shared(t)), g.temp(shared(t)));
                g.assign(Place::local(ra), Rvalue::Ref(0, false, a));
                g.assign(Place::local(rb), Rvalue::Ref(0, false, b));
                let call = Rvalue::Call(
                    Operand::Const(Const::Fn(f, Vec::new())),
                    vec![
                        Operand::Move(Place::local(ra)),
                        Operand::Move(Place::local(rb)),
                    ],
                );
                g.assign(Place::local(r), call);
                g.push(StmtKind::Dead(rb));
                g.push(StmtKind::Dead(ra));
            }
        }
        r
    }

    /// Compares the pairs of places, each of its type, in order: the
    /// result is `false` at the first that differs, else `true`.
    fn all_equal(&mut self, g: &mut G, pairs: Vec<(Type, Place, Place)>, join: usize) {
        for (t, a, b) in pairs {
            let r = self.compare(g, &t, a, b);
            let (next, differ) = (g.new_block(), g.new_block());
            g.terminate(Term::Branch(
                Operand::Copy(Place::local(r)),
                Bb(next),
                Bb(differ),
            ));
            g.cur = differ;
            g.assign(Place::local(RETURN), bool_const(false));
            g.terminate(Term::Goto(Bb(join)));
            g.cur = next;
        }
        g.assign(Place::local(RETURN), bool_const(true));
        g.terminate(Term::Goto(Bb(join)));
    }

    fn eq_body(&mut self, t: &Type) -> Body {
        let mut g = G::new(Type::Bool, vec![shared(t), shared(t)]);
        let a = Place::local(Local(1)).project(Proj::Deref);
        let b = Place::local(Local(2)).project(Proj::Deref);
        let join = g.new_block();
        match t {
            // As a function value, `Eq::eq` of what's compared directly.
            t if scalar(t) => self.all_equal(&mut g, vec![(t.clone(), a, b)], join),
            Type::Ref(_, _, inner) | Type::Box(inner) => {
                let pairs = vec![(
                    (**inner).clone(),
                    a.project(Proj::Deref),
                    b.project(Proj::Deref),
                )];
                self.all_equal(&mut g, pairs, join);
            }
            Type::Array(elem, n) => {
                let Type::Nat(n) = **n else {
                    unreachable!("an array of no length")
                };
                let pairs = (0..n as usize)
                    .map(|i| {
                        let f = Proj::Field(i);
                        ((**elem).clone(), a.project(f), b.project(f))
                    })
                    .collect();
                self.all_equal(&mut g, pairs, join);
            }
            Type::Tuple(ts) => {
                let pairs = (ts.iter().enumerate())
                    .map(|(i, f)| {
                        (
                            f.clone(),
                            a.project(Proj::Field(i)),
                            b.project(Proj::Field(i)),
                        )
                    })
                    .collect();
                self.all_equal(&mut g, pairs, join);
            }
            Type::Slice(elem) => self.slice_eq(
                &mut g,
                elem,
                Operand::Copy(Place::local(Local(1))),
                Operand::Copy(Place::local(Local(2))),
                join,
            ),
            Type::Str => {
                // A string is its bytes.
                let bytes = Type::Slice(Box::new(Type::Int(crate::types::IntTy::U8)));
                let f = self.inherent(&Type::Str, "as_bytes", Vec::new());
                let mut sides = Vec::new();
                for side in [Local(1), Local(2)] {
                    let s = g.temp(shared(&bytes));
                    let call = Rvalue::Call(
                        Operand::Const(f.clone()),
                        vec![Operand::Copy(Place::local(side))],
                    );
                    g.assign(Place::local(s), call);
                    sides.push(s);
                }
                let (sa, sb) = (sides[0], sides[1]);
                let u8 = Type::Int(crate::types::IntTy::U8);
                self.slice_eq(
                    &mut g,
                    &u8,
                    Operand::Copy(Place::local(sa)),
                    Operand::Copy(Place::local(sb)),
                    join,
                );
            }
            Type::Adt(..) => match self.variant_types(t) {
                None => {
                    let fields = self.field_types(t);
                    let pairs = (fields.into_iter().enumerate())
                        .map(|(i, f)| (f, a.project(Proj::Field(i)), b.project(Proj::Field(i))))
                        .collect();
                    self.all_equal(&mut g, pairs, join);
                }
                Some(variants) => {
                    // Which variant `a` is, then whether `b` is it too.
                    let start = g.cur;
                    let mut pats = Vec::new();
                    let mut targets = Vec::new();
                    for (v, fields) in variants.iter().enumerate() {
                        let pat = Pat::Variant(v, vec![Pat::Any; fields.len()]);
                        g.cur = g.new_block();
                        targets.push(Bb(g.cur));
                        pats.push(pat.clone());
                        let (same, other) = (g.new_block(), g.new_block());
                        g.terminate(Term::Match(
                            b.clone(),
                            vec![pat, Pat::Any],
                            vec![Bb(same), Bb(other)],
                        ));
                        g.cur = other;
                        g.assign(Place::local(RETURN), bool_const(false));
                        g.terminate(Term::Goto(Bb(join)));
                        g.cur = same;
                        let pairs = (fields.iter().enumerate())
                            .map(|(i, f)| {
                                let p = Proj::Variant(v, i);
                                (f.clone(), a.project(p), b.project(p))
                            })
                            .collect();
                        self.all_equal(&mut g, pairs, join);
                    }
                    g.cur = start;
                    g.terminate(Term::Match(a.clone(), pats, targets));
                }
            },
            t => unreachable!("`==` glue of `{t}`"),
        }
        g.cur = join;
        g.finish()
    }

    /// The `cmp` of `t`'s `Ord`: its own, by name, or else its glue.
    fn cmp_fn(&mut self, t: &Type) -> String {
        let t = t.erased();
        match self.target(&format!("<{ORD}>::cmp"), vec![t.clone()]) {
            Target::Fn(of, args, copy) => self.m.named_copy(&of, args, copy),
            Target::Cmp(t) => self.cmp_glue(&t),
            Target::Eq(_) | Target::Discard(_) | Target::Debug(..) => {
                unreachable!("`cmp` of `Ord`")
            }
        }
    }

    /// The name of the `cmp` glue of `t`, a derived `Ord`'s, made if it
    /// isn't yet.
    pub(super) fn cmp_glue(&mut self, t: &Type) -> String {
        let t = t.erased();
        let name = glue_name(CMP_GLUE, &t);
        if self.m.glue_made.insert(name.clone()) {
            let body = self.cmp_body(&t);
            self.m.bodies.insert(name.clone(), body);
        }
        name
    }

    /// `Ordering`'s type, and its variants' indices: `Less`, `Equal` and
    /// `Greater`.
    fn ordering(&self) -> (Type, [usize; 3]) {
        let ordering = crate::check::ORDERING;
        let crate::ast::TypeDecl::Enum(e) = self.m.types[ordering] else {
            unreachable!("`Ordering` is an enum")
        };
        let at = |name: &str| {
            (e.variants.iter())
                .position(|v| v.name.name == name)
                .expect("`Ordering`'s variants")
        };
        let ty = Type::Adt(ordering.into(), Vec::new(), Vec::new());
        (ty, [at("Less"), at("Equal"), at("Greater")])
    }

    /// Compares the pairs of places, each of its type, in order: the
    /// result is the first `cmp` that isn't `Equal`, else `Equal`.
    fn all_cmp(&mut self, g: &mut G, pairs: Vec<(Type, Place, Place)>, join: usize) {
        let (ordering, [_, equal, _]) = self.ordering();
        for (t, a, b) in pairs {
            let f = self.cmp_fn(&t);
            let (ra, rb) = (g.temp(shared(&t)), g.temp(shared(&t)));
            g.assign(Place::local(ra), Rvalue::Ref(0, false, a));
            g.assign(Place::local(rb), Rvalue::Ref(0, false, b));
            let r = g.temp(ordering.clone());
            let call = Rvalue::Call(
                Operand::Const(Const::Fn(f, Vec::new())),
                vec![
                    Operand::Move(Place::local(ra)),
                    Operand::Move(Place::local(rb)),
                ],
            );
            g.assign(Place::local(r), call);
            g.push(StmtKind::Dead(rb));
            g.push(StmtKind::Dead(ra));
            let (next, differ) = (g.new_block(), g.new_block());
            g.push(StmtKind::Inspect(Place::local(r)));
            g.terminate(Term::Match(
                Place::local(r),
                vec![Pat::Variant(equal, Vec::new()), Pat::Any],
                vec![Bb(next), Bb(differ)],
            ));
            g.cur = differ;
            g.assign(
                Place::local(RETURN),
                Rvalue::Use(Operand::Move(Place::local(r))),
            );
            g.terminate(Term::Goto(Bb(join)));
            g.cur = next;
        }
        let eq = Rvalue::Aggregate(Agg::Variant(equal), ordering, Vec::new());
        g.assign(Place::local(RETURN), eq);
        g.terminate(Term::Goto(Bb(join)));
    }

    fn cmp_body(&mut self, t: &Type) -> Body {
        let (ordering, [less, _, greater]) = self.ordering();
        let mut g = G::new(ordering.clone(), vec![shared(t), shared(t)]);
        let a = Place::local(Local(1)).project(Proj::Deref);
        let b = Place::local(Local(2)).project(Proj::Deref);
        let join = g.new_block();
        let variants = self.variant_types(t);
        match (t, variants) {
            (Type::Tuple(ts), _) => {
                let pairs = (ts.iter().enumerate())
                    .map(|(i, f)| {
                        (
                            f.clone(),
                            a.project(Proj::Field(i)),
                            b.project(Proj::Field(i)),
                        )
                    })
                    .collect();
                self.all_cmp(&mut g, pairs, join);
            }
            (Type::Adt(..), None) => {
                let pairs = (self.field_types(t).into_iter().enumerate())
                    .map(|(i, f)| (f, a.project(Proj::Field(i)), b.project(Proj::Field(i))))
                    .collect();
                self.all_cmp(&mut g, pairs, join);
            }
            // Which variant `a` is, then which `b` is: an earlier one is
            // less, a later one greater, and the same one compares fields.
            (Type::Adt(..), Some(variants)) => {
                let by = |g: &mut G, v: usize| {
                    let block = g.new_block();
                    let saved = g.cur;
                    g.cur = block;
                    let r = Rvalue::Aggregate(Agg::Variant(v), ordering.clone(), Vec::new());
                    g.assign(Place::local(RETURN), r);
                    g.terminate(Term::Goto(Bb(join)));
                    g.cur = saved;
                    Bb(block)
                };
                let (is_less, is_greater) = (by(&mut g, less), by(&mut g, greater));
                let start = g.cur;
                let mut pats = Vec::new();
                let mut targets = Vec::new();
                for (v, fields) in variants.iter().enumerate() {
                    g.cur = g.new_block();
                    targets.push(Bb(g.cur));
                    pats.push(Pat::Variant(v, vec![Pat::Any; fields.len()]));
                    let same = g.new_block();
                    let bpats = (variants.iter().enumerate())
                        .map(|(w, fs)| Pat::Variant(w, vec![Pat::Any; fs.len()]))
                        .collect();
                    let btargets = (0..variants.len())
                        .map(|w| match w.cmp(&v) {
                            std::cmp::Ordering::Less => is_greater,
                            std::cmp::Ordering::Equal => Bb(same),
                            std::cmp::Ordering::Greater => is_less,
                        })
                        .collect();
                    g.push(StmtKind::Inspect(b.clone()));
                    g.terminate(Term::Match(b.clone(), bpats, btargets));
                    g.cur = same;
                    let pairs = (fields.iter().enumerate())
                        .map(|(i, f)| {
                            let p = Proj::Variant(v, i);
                            (f.clone(), a.project(p), b.project(p))
                        })
                        .collect();
                    self.all_cmp(&mut g, pairs, join);
                }
                g.cur = start;
                g.push(StmtKind::Inspect(a.clone()));
                g.terminate(Term::Match(a.clone(), pats, targets));
            }
            (t, _) => unreachable!("`cmp` glue of `{t}`"),
        }
        g.cur = join;
        g.finish()
    }

    /// `==` on slices `a` and `b`, references to `[elem]`s: of the same
    /// length, and with equal values at each index, from the first.
    fn slice_eq(&mut self, g: &mut G, elem: &Type, a: Operand, b: Operand, join: usize) {
        let usize_t = Type::Int(crate::types::IntTy::Usize);
        let len = self.inherent(
            &Type::Slice(Box::new(Type::Param("T".into(), false))),
            "len",
            vec![elem.clone()],
        );
        let index = self.inherent(
            &Type::Slice(Box::new(Type::Param("T".into(), false))),
            "index",
            vec![elem.clone()],
        );
        let (n, m, i) = (
            g.temp(usize_t.clone()),
            g.temp(usize_t.clone()),
            g.temp(usize_t.clone()),
        );
        for (l, s) in [(n, &a), (m, &b)] {
            let call = Rvalue::Call(Operand::Const(len.clone()), vec![s.clone()]);
            g.assign(Place::local(l), call);
        }
        let same_len = g.temp(Type::Bool);
        let id = self.m.id();
        let value = Rvalue::Binary(
            BinOp::Eq,
            Operand::Copy(Place::local(n)),
            Operand::Copy(Place::local(m)),
            id,
        );
        g.assign(Place::local(same_len), value);
        let (lengths_differ, start) = (g.new_block(), g.new_block());
        g.terminate(Term::Branch(
            Operand::Copy(Place::local(same_len)),
            Bb(start),
            Bb(lengths_differ),
        ));
        g.cur = lengths_differ;
        g.assign(Place::local(RETURN), bool_const(false));
        g.terminate(Term::Goto(Bb(join)));
        g.cur = start;
        g.assign(
            Place::local(i),
            Rvalue::Use(Operand::Const(Const::Int(0, usize_t.clone()))),
        );
        let head = g.new_block();
        g.terminate(Term::Goto(Bb(head)));
        g.cur = head;
        let done = g.temp(Type::Bool);
        let id = self.m.id();
        let value = Rvalue::Binary(
            BinOp::Eq,
            Operand::Copy(Place::local(i)),
            Operand::Copy(Place::local(n)),
            id,
        );
        g.assign(Place::local(done), value);
        let (finished, body) = (g.new_block(), g.new_block());
        g.terminate(Term::Branch(
            Operand::Copy(Place::local(done)),
            Bb(finished),
            Bb(body),
        ));
        g.cur = finished;
        g.assign(Place::local(RETURN), bool_const(true));
        g.terminate(Term::Goto(Bb(join)));
        g.cur = body;
        let mut at = Vec::new();
        for s in [&a, &b] {
            let r = g.temp(shared(elem));
            let call = Rvalue::Call(
                Operand::Const(index.clone()),
                vec![s.clone(), Operand::Copy(Place::local(i))],
            );
            g.assign(Place::local(r), call);
            at.push(r);
        }
        let r = self.compare(
            g,
            elem,
            Place::local(at[0]).project(Proj::Deref),
            Place::local(at[1]).project(Proj::Deref),
        );
        let (next, differ) = (g.new_block(), g.new_block());
        g.terminate(Term::Branch(
            Operand::Copy(Place::local(r)),
            Bb(next),
            Bb(differ),
        ));
        g.cur = differ;
        g.assign(Place::local(RETURN), bool_const(false));
        g.terminate(Term::Goto(Bb(join)));
        g.cur = next;
        // `i < n`, so `i + 1` fits.
        let id = self.m.id();
        let value = Rvalue::Binary(
            BinOp::Add,
            Operand::Copy(Place::local(i)),
            Operand::Const(Const::Int(1, usize_t)),
            id,
        );
        g.assign(Place::local(i), value);
        g.terminate(Term::Goto(Bb(head)));
    }

    /// Inherent method `method` of the impl for `self_ty`, written with its
    /// impl's type parameters, at type arguments `args`, as a value.
    fn inherent(&mut self, self_ty: &Type, method: &str, args: Vec<Type>) -> Const {
        let traits = &self.m.info.traits;
        let symbol = (traits.impls.iter())
            .filter(|d| d.trait_name.is_none() && same_shape(&d.self_ty, self_ty))
            .find_map(|d| d.methods.iter().find(|(m, _)| m == method))
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| unreachable!("`{self_ty}` has a method `{method}`"));
        self.fn_const(&symbol, args)
    }

    /// The function closure type `t`, which captures nothing, is: its
    /// glue, made if it isn't yet. The closure's body is copied in `root`,
    /// the copy of the function it's made in, if it isn't yet.
    pub(super) fn fn_glue(&mut self, t: &Type, root: &str) -> String {
        let t = t.erased();
        let name = glue_name(FN_GLUE, &t);
        if !self.m.glue_made.insert(name.clone()) {
            return name;
        }
        let Type::Adt(symbol, ..) = &t else {
            unreachable!("a closure type is a struct's")
        };
        let def = &self.m.info.traits.closures[symbol];
        let lambda = def.lambda;
        let Type::Fn(sig) = def.sig_at(&t).erased() else {
            unreachable!("a signature is a function type")
        };
        debug_assert!(def.fields.is_empty(), "a reified closure captures nothing");
        self.made_closure(lambda, &t, root);
        let n = sig.params.len();
        let mut g = G::new(*sig.ret, sig.params);
        let c = g.temp(t.clone());
        g.assign(
            Place::local(c),
            Rvalue::Aggregate(Agg::Closure(lambda), t.clone(), Vec::new()),
        );
        let r = g.temp(shared(&t));
        g.assign(Place::local(r), Rvalue::Ref(0, false, Place::local(c)));
        let mut args = vec![Operand::Move(Place::local(r))];
        args.extend((1..=n).map(|i| Operand::Move(Place::local(Local(i)))));
        g.assign(
            Place::local(RETURN),
            Rvalue::Call(Operand::Const(Const::Receiver(FnKind::Fn)), args),
        );
        let body = g.finish();
        self.m.bodies.insert(name.clone(), body);
        name
    }

    /// The call shim of a closure called by value whose body takes it by
    /// reference: it's put in a temporary, called by reference, and
    /// dropped after. `None` if `recv`, of type `t`, needs none.
    pub(super) fn by_ref_body(&self, t: &Type) -> Option<FnKind> {
        let Type::Adt(symbol, ..) = t else {
            return None;
        };
        let c = self.m.info.traits.closures.get(symbol)?;
        match c.kind? {
            FnKind::FnOnce => None,
            k => Some(k),
        }
    }
}

/// A `{debug}` body being written: the value's place, the sink, the
/// result's type, which variant of it is `Ok`, and where it returns from.
struct Dbg {
    w: Local,
    result: Type,
    ok: usize,
    join: usize,
}

impl Cx<'_, '_, '_> {
    /// The name of the `Debug::fmt` glue of `t` into sink `w`, made if it
    /// isn't yet.
    pub(super) fn debug_glue(&mut self, t: &Type, w: &Type) -> String {
        let (t, w) = (t.erased(), w.erased());
        let name = format!("{DEBUG_GLUE}::<{}, {}>", t.symbols(), w.symbols());
        if self.m.glue_made.insert(name.clone()) {
            let body = self.debug_body(&t, &w);
            self.m.bodies.insert(name.clone(), body);
        }
        name
    }

    fn debug_body(&mut self, t: &Type, w: &Type) -> Body {
        let error = Type::Proj(Box::new(w.clone()), WRITE.into(), "Error".into());
        let error = self.m.proj.reveal(&error).erased();
        let result = Type::Adt(
            crate::check::RESULT.into(),
            Vec::new(),
            vec![Type::Unit, error],
        );
        let ok = match self.m.types[crate::check::RESULT] {
            crate::ast::TypeDecl::Enum(e) => (e.variants.iter())
                .position(|v| v.name.name == "Ok")
                .expect("`Result` has `Ok`"),
            _ => unreachable!("`Result` is an enum"),
        };
        let sink = Type::Ref(Region::Erased, true, Box::new(w.clone()));
        let mut g = G::new(result.clone(), vec![shared(t), sink]);
        let join = g.new_block();
        let d = Dbg {
            w: Local(2),
            result,
            ok,
            join,
        };
        let x = Place::local(Local(1)).project(Proj::Deref);
        // `(1, 'a')`: its values in parentheses, with no name.
        if let Type::Tuple(ts) = t {
            let parts = (ts.iter().enumerate())
                .map(|(i, f)| (None, f.clone(), x.project(Proj::Field(i))))
                .collect();
            self.debug_parts(&mut g, &d, "", parts, false);
            g.cur = join;
            return g.finish();
        }
        let Type::Adt(symbol, ..) = t else {
            unreachable!("a derived `Debug` is of a struct or enum")
        };
        let decl = self.m.types[symbol.as_str()];
        match (decl, self.variant_types(t)) {
            (crate::ast::TypeDecl::Enum(e), Some(variants)) => {
                let names: Vec<String> = e.variants.iter().map(|v| v.name.name.clone()).collect();
                let start = g.cur;
                let mut pats = Vec::new();
                let mut targets = Vec::new();
                for (v, fields) in variants.iter().enumerate() {
                    g.cur = g.new_block();
                    targets.push(Bb(g.cur));
                    pats.push(Pat::Variant(v, vec![Pat::Any; fields.len()]));
                    let parts = (fields.iter().enumerate())
                        .map(|(i, f)| (None, f.clone(), x.project(Proj::Variant(v, i))))
                        .collect();
                    self.debug_parts(&mut g, &d, &names[v], parts, false);
                }
                g.cur = start;
                g.push(StmtKind::Inspect(x.clone()));
                g.terminate(Term::Match(x.clone(), pats, targets));
            }
            (crate::ast::TypeDecl::Struct(s), None) => {
                let names: Vec<String> = s.fields.iter().map(|f| f.name.name.clone()).collect();
                let parts = (self.field_types(t).into_iter().enumerate())
                    .map(|(i, f)| (Some(names[i].clone()), f, x.project(Proj::Field(i))))
                    .collect();
                // By its own name, without the path to it.
                let name = s.name.name.rsplit("::").next().unwrap_or(&s.name.name);
                self.debug_parts(&mut g, &d, name, parts, true);
            }
            _ => unreachable!("a derived `Debug` of `{t}`"),
        }
        g.cur = join;
        g.finish()
    }

    /// Writes `name`, then the parts, each `(field name, type, place)`:
    /// `name { a: .., b: .. }` if `braces`, and else `name(.., ..)`; just
    /// `name` if there are none. Then returns what the last write gave.
    fn debug_parts(
        &mut self,
        g: &mut G,
        d: &Dbg,
        name: &str,
        parts: Vec<(Option<String>, Type, Place)>,
        braces: bool,
    ) {
        let (open, close) = match braces {
            true => (" { ", " }"),
            false => ("(", ")"),
        };
        let mut text = name.to_string();
        let n = parts.len();
        for (i, (field, t, place)) in parts.into_iter().enumerate() {
            text.push_str(if i == 0 { open } else { ", " });
            if let Some(f) = field {
                text.push_str(&f);
                text.push_str(": ");
            }
            self.debug_write_str(g, d, std::mem::take(&mut text));
            // `Debug::fmt(&place, &mut *w)`.
            let f = self.fn_const(
                &format!("<{DEBUG}>::fmt"),
                vec![t.clone(), self.sink_of(g, d)],
            );
            let x = g.temp(shared(&t));
            g.assign(Place::local(x), Rvalue::Ref(0, false, place));
            let w = self.reborrow(g, d);
            let call = Rvalue::Call(
                Operand::Const(f),
                vec![
                    Operand::Move(Place::local(x)),
                    Operand::Move(Place::local(w)),
                ],
            );
            self.debug_step(g, d, call);
        }
        if n > 0 {
            text.push_str(close);
        }
        // The last write's result is the body's.
        let f = self.fn_const(&format!("<{WRITE}>::write_str"), vec![self.sink_of(g, d)]);
        let w = self.reborrow(g, d);
        let call = Rvalue::Call(
            Operand::Const(f),
            vec![
                Operand::Move(Place::local(w)),
                Operand::Const(Const::Str(text)),
            ],
        );
        g.assign(Place::local(RETURN), call);
        g.terminate(Term::Goto(Bb(d.join)));
    }

    /// The sink's type, `W`.
    fn sink_of(&self, g: &G, d: &Dbg) -> Type {
        match &g.locals[d.w.0].ty {
            Type::Ref(_, _, w) => (**w).clone(),
            t => unreachable!("a sink `{t}`"),
        }
    }

    /// `&mut *w`, in a new temporary.
    fn reborrow(&self, g: &mut G, d: &Dbg) -> Local {
        let ty = g.locals[d.w.0].ty.clone();
        let w = g.temp(ty);
        let place = Place::local(d.w).project(Proj::Deref);
        g.assign(Place::local(w), Rvalue::Ref(0, true, place));
        w
    }

    /// `w.write_str(text)`, returning its error if it gives one.
    fn debug_write_str(&mut self, g: &mut G, d: &Dbg, text: String) {
        let f = self.fn_const(&format!("<{WRITE}>::write_str"), vec![self.sink_of(g, d)]);
        let w = self.reborrow(g, d);
        let call = Rvalue::Call(
            Operand::Const(f),
            vec![
                Operand::Move(Place::local(w)),
                Operand::Const(Const::Str(text)),
            ],
        );
        self.debug_step(g, d, call);
    }

    /// `call?`: if what it gives is `Ok`, that's used up and the body goes
    /// on; if not, the body returns it.
    fn debug_step(&mut self, g: &mut G, d: &Dbg, call: Rvalue) {
        let r = g.temp(d.result.clone());
        g.assign(Place::local(r), call);
        let (go_on, stop) = (g.new_block(), g.new_block());
        g.push(StmtKind::Inspect(Place::local(r)));
        g.terminate(Term::Match(
            Place::local(r),
            vec![Pat::Variant(d.ok, vec![Pat::Any]), Pat::Any],
            vec![Bb(go_on), Bb(stop)],
        ));
        g.cur = stop;
        g.assign(
            Place::local(RETURN),
            Rvalue::Use(Operand::Move(Place::local(r))),
        );
        g.terminate(Term::Goto(Bb(d.join)));
        g.cur = go_on;
        g.push(StmtKind::Consume(Place::local(r), Some(d.ok)));
    }
}

/// Are `a` and `b` the same type, but for what type parameters stand for?
fn same_shape(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Slice(_), Type::Slice(_)) => true,
        (a, b) => a == b,
    }
}
