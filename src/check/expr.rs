//! Expressions: checking one against the type it must have (`check`), or
//! working out its type (`synth`), with blocks and loops.

use super::info::{Instance, Resolved};
use super::lints::{narrowable, unreachable_alt, unreachable_code, unused_label};
use super::scope::{FnScope, to_closure};
use super::typed::Res;
use super::{Checker, ITERATOR, effects, modules, plural, source_name, traits, unify};
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::exhaust::{self, Redundant};
use crate::traits::Pred;
use crate::types::{FnType, Region, Type, struct_fields, variant_fields};

impl Checker {
    /// Checks that `e` has type `want`.
    ///
    /// Mostly that's "synthesize, then unify". But this is *bidirectional*
    /// typing: checking is its own mode, which pushes the type that's wanted
    /// *down* into the expression, so its parts know their types before
    /// they're looked at.
    ///
    /// That matters for closures: in `apply(|p| p.x, ..)`, `p` gets its type
    /// from `apply`'s signature before the body reads `p.x`, which needs it.
    /// Blocks, `match` and `loop` pass `want` on to their results, so that
    /// reaches closures inside them too.
    ///
    /// A `!` fits whatever is wanted (T-Never).
    pub(super) fn check(&mut self, e: &Expr, want: &Type) -> Result<(), Error> {
        self.here = e.span;
        let got = match &e.kind {
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), Some(want))?,
            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, Some(want))?,
            ExprKind::Loop(label, body) => self.loop_expr(label, None, body, Some(want))?,
            ExprKind::Lambda(l) => {
                let sig = match self.shallow(want) {
                    // A function type is wanted, which a closure can only be
                    // if it captures nothing: what it captures has to be
                    // somewhere.
                    Type::Fn(sig) => match self.captured(l).first() {
                        Some(id) => {
                            let name = &self.typed.binds[id].name;
                            let at = first_use(&l.body, name).unwrap_or(l.body.span);
                            let ps: Vec<String> =
                                sig.params.iter().map(|p| p.to_string()).collect();
                            return Err(Error::ty(
                                e.span,
                                format!(
                                    "expected `{}`, found a closure that captures `{name}`",
                                    Type::Fn(sig.clone())
                                ),
                            )
                            .with_note(at, format!("`{name}` is captured here"))
                            .with_note(
                                e.span,
                                format!(
                                    "a function type only holds functions, and closures that \
                                     capture nothing; to take this, take a type parameter \
                                     `F: Fn({}) -> {}`",
                                    ps.join(", "),
                                    sig.ret
                                ),
                            ));
                        }
                        None if sig.params.len() == l.params.len() => Some(sig),
                        None => None,
                    },
                    // What a bound on a type to be inferred says calls of it
                    // take and give: `F: Fn(int) -> int` for `apply`'s `F`
                    // in `apply(|x| x + 1)`.
                    Type::Var(v) => self.wanted_sig(v, l.params.len()),
                    _ => None,
                };
                self.lambda(l, sig)?
            }
            // `&x` where a `&T` is wanted: `x` should be a `T`.
            ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) => match self.shallow(want) {
                Type::Ref(_, m, inner) if m == (*op == UnOp::RefMut) => {
                    self.check(x, &inner)?;
                    Type::Ref(Region::Erased, m, inner)
                }
                _ => self.synth(e)?,
            },
            _ => self.synth(e)?,
        };
        self.typed.exprs.insert(e.id, got.clone());
        self.note_never(e, &got);
        if let Some(target) = self.coerce(e, &got, want) {
            return match self.unify(&target, want) {
                Ok(()) => Ok(()),
                Err(_) => Err(Error::ty(
                    result_span(e),
                    format!(
                        "expected `{}`, found `{}`",
                        self.solved(want),
                        self.solved(&got)
                    ),
                )),
            };
        }
        // A function gains effects: one that can't panic will do where
        // one that may is wanted, as nothing calling it counts on a panic.
        // Only as a whole value, here, not inside another type: there's no
        // subtyping.
        if let (Type::Fn(g), Type::Fn(w)) = (self.shallow(&got), self.shallow(want)) {
            let bare = |f: &FnType| Type::Fn(f.with_effects(Type::no_effects()));
            let (gs, ws) = (self.solved(&got), self.solved(want));
            self.unify_or(&bare(&g), &bare(&w), result_span(e), |_, _| {
                format!("expected `{ws}`, found `{gs}`")
            })?;
            let why = effects::Why::Coerce {
                got: gs.clone(),
                want: ws.clone(),
            };
            let (ge, we) = (self.solved(&g.effects), self.solved(&w.effects));
            let open = |t: &Type| unify::has_vars(t);
            if !open(&ge) && !open(&we) && !crate::traits::subset(&ge, &we) {
                return Err(Error::ty(
                    result_span(e),
                    format!("expected `{ws}`, found `{gs}`"),
                ));
            }
            self.within(ge, we, result_span(e), why);
            return Ok(());
        }
        self.fits(&got, want, result_span(e), |got, want| {
            format!("expected `{want}`, found `{got}`")
        })
    }

    /// Like `unify_or`, but `got` fits if it's `!`, by T-Never, whatever
    /// `want` is. If `want` is a type not known yet, it's noted, to be `!`
    /// if nothing else decides it (see `solve_all`): in `let x = id(return);`
    /// the type argument of `id` is `!`.
    pub(super) fn fits(
        &mut self,
        got: &Type,
        want: &Type,
        span: Span,
        mismatch: impl FnOnce(&Type, &Type) -> String,
    ) -> Result<(), Error> {
        if self.shallow(got) != Type::Never {
            return self.unify_or(got, want, span, mismatch);
        }
        if let Type::Var(_) = self.shallow(want) {
            self.diverging.push(want.clone());
        }
        Ok(())
    }

    /// The type of two branches of types `a` and `b`, one of which runs:
    /// the other's, if one is `!`, or else both, which must be the same.
    pub(super) fn join(
        &mut self,
        a: &Type,
        b: &Type,
        span: Span,
        mismatch: impl FnOnce(&Type, &Type) -> String,
    ) -> Result<Type, Error> {
        match (self.shallow(a), self.shallow(b)) {
            (Type::Never, _) => Ok(b.clone()),
            (_, Type::Never) => Ok(a.clone()),
            _ => {
                self.unify_or(a, b, span, mismatch)?;
                Ok(a.clone())
            }
        }
    }

    /// Notes that the code checked so far diverges if `e` has type `t`, a
    /// `!`, which has no values to finish with.
    fn note_never(&mut self, e: &Expr, t: &Type) {
        if self.shallow(t) == Type::Never && self.diverges.is_none() {
            self.diverges = Some(e.span);
        }
    }

    /// If `e`, a reference of type `got`, is wanted as a reference of type
    /// `want` that it isn't, the reference it can be *coerced* to: it's
    /// borrowed again, as `&**e` say. A `&mut T` can be used as a `&T`,
    /// since there's nothing a `&T` can do that a `&mut T` can't, and a
    /// `&Box<T>` or `&&T` as a `&T`, by following the box or reference to
    /// what it points to, as Rust's *deref coercion* does. Recorded for the
    /// borrow checker, and for `lift` to write out.
    fn coerce(&mut self, e: &Expr, got: &Type, want: &Type) -> Option<Type> {
        let (Type::Ref(_, gm, a), Type::Ref(_, wm, b)) = (self.shallow(got), self.shallow(want))
        else {
            return None;
        };
        if wm && !gm {
            return None;
        }
        let mut a = *a;
        let mut derefs = 0;
        loop {
            let (sa, sb) = (self.shallow(&a), self.shallow(&b));
            if matches!(sa, Type::Var(_)) || matches!(sb, Type::Var(_)) || sa.same_shape(&sb) {
                break;
            }
            match sa {
                Type::Box(inner) => a = *inner,
                Type::Ref(_, m, inner) if m || !wm => a = *inner,
                _ => break,
            }
            derefs += 1;
        }
        if derefs == 0 && gm == wm {
            return None;
        }
        self.typed.coerced.insert(e.id, (derefs, wm));
        self.info.coercions.insert(e.id, (derefs, wm));
        Some(Type::Ref(Region::Erased, wm, Box::new(a)))
    }

    /// Computes ("synthesizes") the type of `e`.
    pub(super) fn synth(&mut self, e: &Expr) -> Result<Type, Error> {
        self.here = e.span;
        let t = self.synth_inner(e)?;
        self.settle_goals()?;
        self.typed.exprs.insert(e.id, t.clone());
        self.note_never(e, &t);
        Ok(t)
    }

    fn synth_inner(&mut self, e: &Expr) -> Result<Type, Error> {
        match &e.kind {
            ExprKind::Move(_) => unreachable!("written by lift"),
            ExprKind::For(_) | ExprKind::Range(..) => {
                unreachable!("rewritten when paths are resolved")
            }
            //  ----------------- (T-Int)       ------------------- (T-True/False)
            //   Γ ⊢ 42 : int                    Γ ⊢ true : bool
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Bool(_) => Ok(Type::Bool),

            //  -------------- (T-Unit)
            //   Γ ⊢ () : ()
            ExprKind::Unit => Ok(Type::Unit),

            //   (x : T) ∈ Γ
            //  ------------- (T-Var)
            //   Γ ⊢ x : T
            //
            // If x is a generic function, T is its type with the type
            // arguments substituted for its type parameters: the ones written
            // in `x::<..>`, or else a fresh variable for each (see `lookup`).
            ExprKind::Var(name, targs) => {
                let Some((t, res)) = self.lookup(name, targs, e.span, e.id)? else {
                    let unknown = format!("unknown variable `{name}`");
                    // `replace`, which isn't in the prelude: say where it is.
                    let path = self.modules.core_fn(name).filter(|_| !name.contains("::"));
                    let Some(path) = path else {
                        return Err(Error::ty(e.span, unknown));
                    };
                    let fix = Fix::new(
                        format!("import `{path}`"),
                        vec![Edit::insert(
                            self.modules.ends[self.module],
                            format!("\nuse {path};\n"),
                        )],
                    );
                    return Err(Error::ty(
                        e.span,
                        format!(
                            "{unknown}; the core library has `{path}`: import it with `use \
                             {path};`"
                        ),
                    )
                    .with_fix(fix));
                };
                self.typed.vars.insert(e.id, res);
                Ok(t)
            }

            //   Γ ⊢ e : int            Γ ⊢ e : bool
            //  -------------- (T-Neg)  -------------- (T-Not)
            //   Γ ⊢ -e : int            Γ ⊢ !e : bool
            //
            // `-x` panics at the least `int`, which has no positive
            // counterpart. Not `-5`: a literal is at most the greatest
            // `int`, so negating it can't overflow.
            ExprKind::Unary(op @ (UnOp::Neg | UnOp::Not), x) => {
                let t = match op {
                    UnOp::Neg => Type::Int,
                    _ => Type::Bool,
                };
                self.check(x, &t)?;
                if *op == UnOp::Neg && !matches!(x.kind, ExprKind::Int(_)) {
                    self.effect(e.span, "`-`", Type::effects([Type::Panic]));
                }
                Ok(t)
            }

            //        Γ ⊢ e : T                   Γ ⊢ e : T
            //  ---------------------- (T-Ref)  -------------------------- (T-RefMut)
            //   Γ ⊢ &e : &'r T                Γ ⊢ &mut e : &'r mut T
            //
            // The region `'r` is how long the borrow lasts. Nothing here
            // decides that; the borrow checker works it out, and checks that
            // `e` is a place that lives that long and isn't used in a way
            // that clashes with the borrow while it lasts. `&` of a value
            // that isn't a place borrows a temporary holding it.
            ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) => {
                let t = self.synth(x)?;
                Ok(Type::Ref(Region::Erased, *op == UnOp::RefMut, Box::new(t)))
            }

            //   Γ ⊢ e : &'r T         Γ ⊢ e : &'r mut T         Γ ⊢ e : Box<T>
            //  --------------- (T-Deref) ------------------- ------------------
            //    Γ ⊢ *e : T              Γ ⊢ *e : T              Γ ⊢ *e : T
            ExprKind::Unary(UnOp::Deref, x) => {
                let t = self.synth(x)?;
                match self.shallow(&t) {
                    Type::Ref(_, _, inner) | Type::Box(inner) => Ok(*inner),
                    Type::Var(_) => Err(Error::ty(
                        x.span,
                        "the type of this must be known to use `*` on it; add a type annotation",
                    )),
                    t => Err(Error::ty(
                        e.span,
                        format!("can't use `*` on `{t}`: only on a reference or a box"),
                    )),
                }
            }

            //   Γ ⊢ p : T    p is a place    Γ ⊢ e : T
            //  ---------------------------------------- (T-Assign)
            //               Γ ⊢ p = e : ()
            //
            // Whether `p` may be changed, by being declared `mut` or being
            // behind a `&mut`, is for the borrow checker.
            ExprKind::Assign(op, place, value) => {
                let not_place = || {
                    Error::ty(
                        place.span,
                        "can't assign to this: only to a variable, a field, or what `*` points to",
                    )
                };
                if !place.is_place() {
                    return Err(not_place());
                }
                let t = self.synth(place)?;
                if let ExprKind::Var(..) = place.kind
                    && !matches!(self.typed.vars.get(&place.id), Some(Res::Local(_)))
                {
                    return Err(not_place());
                }
                match op {
                    None => {
                        self.check(value, &t)?;
                    }
                    Some(op) => {
                        self.unify_or(&t, &Type::Int, place.span, |t, _| {
                            format!("`{op}=` works on an `int`, not on `{t}`")
                        })?;
                        self.check(value, &Type::Int)?;
                    }
                }
                Ok(Type::Unit)
            }

            //   Γ ⊢ c : bool    Γ ⊢ body : ()
            //  ------------------------------- (T-While)
            //     Γ ⊢ while c { body } : ()
            //
            // The body may not run at all, so whether it diverges doesn't
            // matter to what comes after, and nor does a `break` in it.
            ExprKind::While(label, c, body) => self.loop_expr(label, Some(c), body, None),

            ExprKind::Loop(label, body) => self.loop_expr(label, None, body, None),

            //   Γ ⊢ e : R    R is what the function returns
            //  -------------------------------------------- (T-Return)
            //               Γ ⊢ return e : !
            //
            // `return` is short for `return ()`. Its own type is `!`: it
            // never finishes, as control leaves for the caller.
            ExprKind::Return(value) => {
                let ret = match self.ret.clone() {
                    Some(ret) => ret,
                    None => {
                        let what = "the type of what this closure returns".to_string();
                        let ret = self.fresh(
                            e.span,
                            what,
                            "annotate it, e.g. `|x| -> int { .. }`".into(),
                        );
                        self.ret = Some(ret.clone());
                        ret
                    }
                };
                match value {
                    Some(v) => {
                        self.check(v, &ret)?;
                    }
                    None => self.unify_or(&Type::Unit, &ret, e.span, |_, ret| {
                        format!("a `return` with no value returns `()`, but this returns `{ret}`")
                    })?,
                }
                Ok(Type::Never)
            }

            //   Γ ⊢ e : T    the loop around is a `loop` of type T
            //  --------------------------------------------------- (T-Break)
            //                 Γ ⊢ break e : !
            //
            //   the loop around is a `loop` of type () or a `while`
            //  ---------------------------------------------------- (T-Break-Unit)
            //                  Γ ⊢ break : !
            //
            //   there's a loop around
            //  ----------------------- (T-Continue)
            //   Γ ⊢ continue : !
            //
            // The loop is the innermost, or with a label, the one with it.
            ExprKind::Break(label, value) => {
                let i = self.target(e.span, "break", label.as_ref())?;
                let Target::Loop(lp) = &self.loops[i] else {
                    unreachable!("a loop")
                };
                if lp.is_while && value.is_some() {
                    return Err(Error::ty(
                        e.span,
                        "a `break` with a value only works in a `loop`: a `while` loop can end \
                         without a `break`, and then it has no value to give",
                    ));
                }
                let target = lp.value.clone().or_else(|| lp.want.clone());
                let t = match (value, target) {
                    (Some(v), Some(t)) => {
                        self.check(v, &t)?;
                        Some(t)
                    }
                    (Some(v), None) => Some(self.synth(v)?),
                    (None, target) => {
                        if let Some(t) = &target {
                            self.unify_or(&Type::Unit, t, e.span, |_, t| {
                                format!(
                                    "a `break` with no value gives the loop `()`, but it should \
                                     be `{t}`"
                                )
                            })?;
                        }
                        Some(Type::Unit)
                    }
                };
                // A value of type `!` never gets as far as the `break`.
                let t = t.filter(|t| self.shallow(t) != Type::Never);
                if let (Some(t), Target::Loop(lp)) = (t, &mut self.loops[i])
                    && lp.value.is_none()
                {
                    lp.value = Some(t);
                }
                Ok(Type::Never)
            }
            ExprKind::Continue(label) => {
                self.target(e.span, "continue", label.as_ref())?;
                Ok(Type::Never)
            }

            ExprKind::Binary(op, l, r) => self.binary(e, op, l, r),
            ExprKind::Call(f, args) => self.call(e, f, args),
            ExprKind::MethodCall(x, name, targs, args) => self.method_call(e, x, name, targs, args),

            //   Γ ⊢ c : bool   Γ ⊢ a : T   Γ ⊢ b : T
            //  --------------------------------------- (T-If)
            //        Γ ⊢ if c { a } else { b } : T
            //
            //   Γ ⊢ c : bool   Γ ⊢ a : ()
            //  --------------------------- (T-If-NoElse)
            //     Γ ⊢ if c { a } : ()
            //
            // If one branch is a `!`, the `if` has the other's type (T-Never).
            // It diverges if both branches do.
            ExprKind::If(c, then, els) => {
                self.check(c, &Type::Bool)?;
                let before = self.diverges.take();
                let t = match els {
                    Some(els) => {
                        let t = self.synth(then)?;
                        let then_diverges = self.diverges.take();
                        let u = self.synth(els)?;
                        let both = then_diverges.is_some() && self.diverges.is_some();
                        self.diverges = before.or(both.then_some(e.span));
                        self.join(&t, &u, result_span(els), |t, u| {
                            format!("`if` and `else` have different types: `{t}` and `{u}`")
                        })?
                    }
                    None => {
                        let t = self.synth(then)?;
                        self.diverges = before;
                        self.fits(&t, &Type::Unit, result_span(then), |t, _| {
                            format!("`if` without `else` must have type `()`, found `{t}`")
                        })?;
                        Type::Unit
                    }
                };
                Ok(t)
            }

            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), None),

            //   struct S { f1: T1, .., fn: Tn }    Γ ⊢ e1 : T1  ..  Γ ⊢ en : Tn
            //  ---------------------------------------------------------------- (T-Struct)
            //               Γ ⊢ S { f1: e1, .., fn: en } : S
            //
            // A product type is *introduced* by giving every field.
            //
            // For a generic struct, S is `S<A1, .., Ak>`, with the type
            // arguments written or a fresh variable for each type parameter,
            // and T1..Tn mention them.
            ExprKind::StructLit(name, targs, inits) => {
                let ty = self.struct_type(name, targs)?;
                let Type::Adt(symbol, ..) = &ty else {
                    unreachable!("a struct")
                };
                let symbol = symbol.clone();
                let fields = struct_fields(&self.adts, &ty).expect("a struct");
                let mut given = vec![false; fields.len()];
                for init in inits {
                    let Some(i) = fields.iter().position(|(f, _)| *f == init.name.name) else {
                        return Err(Error::ty(
                            init.name.span,
                            format!("`{}` has no field `{}`", name.name, init.name.name),
                        ));
                    };
                    if given[i] {
                        return Err(Error::ty(
                            init.name.span,
                            format!("field `{}` is given twice", init.name.name),
                        ));
                    }
                    given[i] = true;
                    self.field_visible(&symbol, &init.name)?;
                    self.note_field(&symbol, &init.name, &fields[i].1);
                    self.check(&init.value, &fields[i].1)?;
                }
                if let Some(i) = given.iter().position(|g| !g) {
                    return Err(Error::ty(
                        e.span,
                        format!("missing field `{}` in `{}`", fields[i].0, name.name),
                    ));
                }
                self.uses().types.insert(symbol.clone());
                self.uses().structs.insert(symbol);
                Ok(ty)
            }

            //   Γ ⊢ e : S    struct S { .., f: T, .. }
            //  ---------------------------------------- (T-Field)
            //               Γ ⊢ e.f : T
            //
            // ...and *eliminated* by taking any one field out.
            //
            // Unlike most rules, this one needs e's type *now*: which struct
            // it is decides what T is. So it can't be a variable to solve
            // later.
            //
            // If e is a reference or a box, the field is of what it points
            // to, at any depth: `r.x` for `r: &Point` is `(*r).x`.
            ExprKind::Field(x, field) => {
                let t = self.synth(x)?;
                let mut t = self.solved(&t);
                let mut derefs = 0;
                while let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                    t = *inner;
                    derefs += 1;
                }
                if let Type::Var(_) = t {
                    return Err(Error::ty(
                        x.span,
                        format!(
                            "the type of this must be known to read its field `{}`; \
                             add a type annotation",
                            field.name
                        ),
                    ));
                }
                let fields = struct_fields(&self.adts, &t).unwrap_or_default();
                match fields
                    .into_iter()
                    .enumerate()
                    .find(|(_, (f, _))| *f == field.name)
                {
                    Some((i, (_, ft))) => {
                        if let Type::Adt(s, ..) = &t {
                            self.field_visible(s, field)?;
                            self.note_field(&s.clone(), field, &ft);
                        }
                        self.typed.fields.insert(e.id, (derefs, i));
                        Ok(ft)
                    }
                    None => Err(Error::ty(
                        field.span,
                        format!("`{t}` has no field `{}`", field.name),
                    )),
                }
            }

            //   enum E { .., V(T1, .., Tn), .. }    Γ ⊢ e1 : T1  ..  Γ ⊢ en : Tn
            //  ----------------------------------------------------------------- (T-Variant)
            //                   Γ ⊢ E::V(e1, .., en) : E
            //
            // A sum type is *introduced* by picking one variant and giving its
            // fields. Its eliminator is `match`.
            ExprKind::Variant(ty, targs, variant, args, ..) if self.is_box(ty) => {
                self.box_new(e, targs, variant, args)
            }
            ExprKind::Variant(ty, targs, method, args, parens) if !self.is_variant(ty, method) => {
                self.path(e, ty, targs, method, args, *parens)
            }
            ExprKind::Variant(ty, targs, variant, args, ..) => {
                let (t, i) = self.variant(ty, targs, variant)?;
                let fields = variant_fields(&self.adts, &t, i);
                if fields.len() != args.len() {
                    return Err(Error::ty(
                        e.span,
                        format!(
                            "`{}::{}` has {} but {} {} supplied",
                            ty.name,
                            variant.name,
                            plural(fields.len(), "field"),
                            args.len(),
                            if args.len() == 1 { "was" } else { "were" },
                        ),
                    ));
                }
                for (arg, field) in args.iter().zip(&fields) {
                    self.check(arg, field)?;
                }
                let Type::Adt(symbol, ..) = &t else {
                    unreachable!("an enum")
                };
                let (ty, variant) = (symbol.clone(), variant.name.clone());
                self.uses().types.insert(ty.clone());
                self.uses().variants.insert((ty, variant));
                Ok(t)
            }

            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, None),

            ExprKind::Lambda(l) => self.lambda(l, None),
        }
    }

    // `l op r`, by the rule for its operator.
    fn binary(&mut self, e: &Expr, op: &BinOp, l: &Expr, r: &Expr) -> Result<Type, Error> {
        match op {
            //   Γ ⊢ l : int   Γ ⊢ r : int
            //  --------------------------- (T-Arith)   for + - * / %
            //       Γ ⊢ l + r : int
            //
            // Each may panic: on overflow, or `/` and `%` by zero. Not
            // `/` or `%` by a literal other than `0`: a literal is at
            // least 1 then, which can't overflow either, as only the
            // least `int` divided by -1 does.
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                self.check(l, &Type::Int)?;
                self.check(r, &Type::Int)?;
                let by_literal = matches!(op, BinOp::Div | BinOp::Rem)
                    && matches!(r.kind, ExprKind::Int(n) if n != 0);
                if !by_literal {
                    self.effect(e.span, format!("`{op}`"), Type::effects([Type::Panic]));
                }
                Ok(Type::Int)
            }

            //   Γ ⊢ l : int   Γ ⊢ r : int
            //  --------------------------- (T-Cmp)     for < <= > >=
            //       Γ ⊢ l < r : bool
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.check(l, &Type::Int)?;
                self.check(r, &Type::Int)?;
                Ok(Type::Bool)
            }

            //   Γ ⊢ l : bool   Γ ⊢ r : bool
            //  ----------------------------- (T-Logic) for && ||
            //       Γ ⊢ l && r : bool
            //
            // The right side may not run, so whether it diverges doesn't
            // matter to what comes after.
            BinOp::And | BinOp::Or => {
                self.check(l, &Type::Bool)?;
                let before = self.diverges.take();
                self.check(r, &Type::Bool)?;
                self.diverges = before;
                Ok(Type::Bool)
            }

            //   Γ ⊢ l : T   Γ ⊢ r : T   T: Eq
            //  ------------------------------- (T-Eq) for == !=
            //         Γ ⊢ l == r : bool
            //
            // `==` is `Eq::eq(&l, &r)`, and `!=` its negation, so T
            // must implement `Eq`. `int`, `bool` and `()` do, and
            // references and boxes if what they point to does: they
            // compare that, not addresses. A struct or enum does if
            // it says so, with `#[derive(Eq)]` to compare field by
            // field, or an impl of its own. Functions don't, and can't:
            // whether two functions do the same for every argument is
            // undecidable, and comparing them by address would make
            // `==` depend on how the compiler laid out the code. A type
            // parameter `T` does if a bound says so, `T: Eq`.
            //
            // If T isn't fully known yet, as in `|a, b| a == b`, the
            // obligation waits until it is, at the end of the function.
            //
            // Being a call of `eq`, it's a use of it, at `T`: for
            // monomorphisation to make the copies of the impls' `eq`s it
            // may call, and for the check against polymorphic recursion
            // to see it.
            BinOp::Eq | BinOp::Ne => {
                let t = self.synth(l)?;
                self.check(r, &t)?;
                let eq = traits::trait_method_symbol("Eq", "eq");
                self.pending
                    .push((e.id, e.span, Instance::Fn(eq, vec![t.clone()])));
                let pred = Pred {
                    ty: t,
                    trait_name: "Eq".into(),
                };
                self.oblige(pred, e.span, traits::Why::Compare(*op))?;
                Ok(Type::Bool)
            }
        }
    }

    //   Γ ⊢ f : fn(T1, .., Tn) -> R    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
    //  ------------------------------------------------------------ (T-App)
    //                  Γ ⊢ f(a1, .., an) : R
    //
    // If f's type isn't known yet, it's a function: one taking n
    // arguments of some types, returning some type.
    fn call(&mut self, e: &Expr, f: &Expr, args: &[Expr]) -> Result<Type, Error> {
        let ft = self.synth(f)?;
        let what = match &f.kind {
            ExprKind::Var(name, _) => format!("`{name}`"),
            _ => "this call".to_string(),
        };
        let (params, ret) = match self.shallow(&ft) {
            Type::Fn(sig) => {
                self.effect(e.span, what, *sig.effects);
                (sig.params, *sig.ret)
            }
            Type::Var(_) => {
                let params: Vec<Type> = args
                    .iter()
                    .map(|a| {
                        let what = "the type of this argument".to_string();
                        self.fresh(a.span, what, "add a type annotation".into())
                    })
                    .collect();
                let it = "the type of this call".to_string();
                let ret = self.fresh(e.span, it, "add a type annotation".into());
                let effects = self.fresh_effects(e.span, "the effects of this call".into());
                let sig = Type::func_with(params.clone(), ret.clone(), effects.clone());
                self.unify(&ft, &sig)
                    .unwrap_or_else(|_| unreachable!("fresh variables fit anything"));
                self.effect(e.span, what, effects);
                (params, ret)
            }
            // A closure, or what a bound says can be called, or a
            // reference to one: what its signature says. How the
            // call gets at it is worked out once that's known
            // (`settle_calls`).
            t => {
                let sig = Type::Proj(
                    Box::new(t.clone()),
                    "FnOnce".into(),
                    crate::traits::SIG.into(),
                );
                let Type::Fn(sig) = self.shallow(&sig) else {
                    let hint = match &t {
                        Type::Param(p) => format!(
                            "; to call it, say what it takes and gives with a bound, \
                             like `{p}: Fn(int) -> int`"
                        ),
                        _ => String::new(),
                    };
                    return Err(Error::ty(f.span, format!("`{t}` is not a function{hint}")));
                };
                let pred = Checker::callable_pred(&t);
                self.oblige(pred, f.span, traits::Why::Call)?;
                self.calls.push((e.id, t, e.span));
                self.effect(e.span, what, *sig.effects);
                (sig.params, *sig.ret)
            }
        };
        if params.len() != args.len() {
            return Err(Error::ty(
                e.span,
                format!(
                    "this function takes {} argument{} but {} {} supplied",
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" },
                ),
            ));
        }
        for (arg, param) in args.iter().zip(&params) {
            self.check(arg, param)?;
        }
        Ok(ret)
    }

    //   Γ ⊢ e : &..&S    m is a method of S of type fn(R, T1, .., Tn) -> U
    //   R is S, &S or &mut S    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
    //  ------------------------------------------------------------------ (T-Method)
    //                   Γ ⊢ e.m(a1, .., an) : U
    //
    // A method call is a call of the method with `e` as its first
    // argument, its *receiver*: what `e` points to, through any
    // number of references and boxes, as a field is (T-Field), and
    // then borrowed, if the method takes `&self` or `&mut self`. So
    // `v.push(1)` is `<V>::push(&mut v, 1)`, and for `r: &&V`,
    // `r.len()` is `<V>::len(&**r)`. That's all the adjusting there
    // is: at most one `&` or `&mut`.
    //
    // Which method `m` is depends on S, so S must be known here, as
    // for a field. See `lookup_method` for how it's found.
    fn method_call(
        &mut self,
        e: &Expr,
        x: &Expr,
        name: &Ident,
        targs: &[TypeExpr],
        args: &[Expr],
    ) -> Result<Type, Error> {
        let t = self.synth(x)?;
        let mut base = self.solved(&t);
        let mut derefs = 0;
        while let Type::Ref(_, _, inner) | Type::Box(inner) = base {
            base = *inner;
            derefs += 1;
        }
        if let Type::Var(_) = base {
            return Err(Error::ty(
                x.span,
                format!(
                    "the type of this must be known to call its method `{}`; add a \
                     type annotation",
                    name.name
                ),
            ));
        }
        let found = match self.method_of(&base, name, targs) {
            Ok(found) => found,
            // `s.f(1)` for a field `f` that holds a function.
            Err(err) => match struct_fields(&self.adts, &base)
                .and_then(|fs| fs.into_iter().find(|(f, _)| *f == name.name))
            {
                Some((_, Type::Fn(_))) => {
                    let field = Span::new(x.span.start, name.span.end);
                    return Err(Error::ty(
                        name.span,
                        format!(
                            "`{base}` has no method `{0}`, but it has a field `{0}`, which \
                             holds a function; to call that, put the field in \
                             parentheses",
                            name.name
                        ),
                    )
                    .with_fix(Fix::new(
                        "put the field in parentheses",
                        vec![Edit::insert(field.start, "("), Edit::insert(field.end, ")")],
                    )));
                }
                _ => return Err(err),
            },
        };
        if !found.receiver {
            return Err(Error::ty(
                name.span,
                format!(
                    "`{}` doesn't take `self`, so it can't be called with `.`; call it \
                     as `{}::{}(..)`",
                    name.name,
                    source_name(&base.to_string()),
                    name.name
                ),
            ));
        }
        let Type::Fn(sig) = &found.ty else {
            unreachable!("a method has a function type")
        };
        let autoref = match self.shallow(&sig.params[0]) {
            Type::Ref(_, m, _) => Some(m),
            _ => None,
        };
        let receiver = match autoref {
            Some(m) => Type::Ref(Region::Erased, m, Box::new(base.clone())),
            None => base.clone(),
        };
        self.unify(&receiver, &sig.params[0])
            .unwrap_or_else(|_| unreachable!("the method is of what's before the `.`"));
        let params = &sig.params[1..];
        if params.len() != args.len() {
            return Err(Error::ty(
                e.span,
                format!(
                    "method `{}` takes {} but {} {} supplied",
                    name.name,
                    plural(params.len(), "argument"),
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" },
                ),
            ));
        }
        let ret = (*sig.ret).clone();
        let params = params.to_vec();
        self.effect(e.span, format!("`{}`", name.name), (*sig.effects).clone());
        self.resolved(name, &found, targs.is_empty());
        for (arg, param) in args.iter().zip(&params) {
            self.check(arg, param)?;
        }
        let res = Res::Fn(found.symbol.clone(), found.args.clone());
        self.typed.methods.insert(e.id, (res, derefs, autoref));
        let resolved = Resolved {
            symbol: found.symbol.clone(),
            derefs,
            autoref,
        };
        self.info.methods.insert(name.id, resolved);
        if let Some(pred) = found.of_trait {
            self.oblige(pred, name.span, traits::Why::Method(name.name.clone()))?;
        }
        Ok(ret)
    }

    //   Γ ⊢ body : ()    the `break`s in body give T
    //  ---------------------------------------------- (T-Loop)
    //            Γ ⊢ loop { body } : T
    //
    //   Γ ⊢ body : ()    no `break` in body gets out
    //  -------------------------------------------- (T-Loop-Never)
    //            Γ ⊢ loop { body } : !
    //
    // The only way out of a `loop` is a `break`, so its value is what they
    // give it. With none, it never ends.
    //
    // `loop_expr` checks a `while` too, with its condition `cond`: T-While.
    // A `break` in the condition is for a loop around the `while`, as the
    // condition isn't part of its body. A loop's label can't be one a loop
    // around it has, which `break` couldn't get to past it, and one that
    // nothing uses gets a warning.
    fn loop_expr(
        &mut self,
        label: &Option<Label>,
        cond: Option<&Expr>,
        body: &Expr,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        if let Some(c) = cond {
            self.check(c, &Type::Bool)?;
        }
        if let Some(l) = label {
            let outer = self.loops.iter().rev().map_while(|t| match t {
                Target::Loop(lp) => Some(lp),
                Target::Closure => None,
            });
            if let Some(outer) = outer
                .filter_map(|lp| lp.label.as_ref())
                .find(|o| o.name.name == l.name.name)
            {
                let name = &l.name.name;
                return Err(Error::ty(
                    l.name.span,
                    format!(
                        "a loop around this one is labelled `{name}` already, so a `break {name}` \
                         in here couldn't get to it; give this one another label"
                    ),
                )
                .with_note(outer.name.span, format!("`{name}` is that loop's label")));
            }
        }
        let before = self.diverges.take();
        self.loops.push(Target::Loop(Box::new(LoopCx {
            label: label.clone(),
            used: false,
            is_while: cond.is_some(),
            want: want.cloned(),
            value: None,
        })));
        let what = match cond {
            Some(_) => "a `while` loop's body",
            None => "a `loop`'s body",
        };
        let result = self.loop_body(body, what);
        let Some(Target::Loop(lp)) = self.loops.pop() else {
            unreachable!("pushed a loop")
        };
        self.diverges = before;
        result?;
        if let (Some(l), false) = (&lp.label, lp.used) {
            self.warnings.push(unused_label(l));
        }
        Ok(match cond {
            Some(_) => Type::Unit,
            None => lp.value.unwrap_or(Type::Never),
        })
    }

    /// Checks the body of a `while` or `loop`, which must be a `()`, as a
    /// value would have nowhere to go. `what` names it in the message.
    fn loop_body(&mut self, body: &Expr, what: &str) -> Result<(), Error> {
        let t = self.synth(body)?;
        self.fits(&t, &Type::Unit, result_span(body), |t, _| {
            format!(
                "expected `()`, found `{t}`: {what} has nowhere for a value to go; add `;` to \
                 discard it"
            )
        })
    }

    /// The index in `loops` of the loop a `break` or `continue` (`what`) at
    /// `span` goes to: the innermost, or the one with `label`, which is then
    /// used.
    fn target(&mut self, span: Span, what: &str, label: Option<&Ident>) -> Result<usize, Error> {
        let closure = || {
            Error::ty(
                span,
                format!(
                    "can't `{what}` out of a closure: its body is a function of its own, so the \
                     loop around it isn't its to leave"
                ),
            )
        };
        let Some(label) = label else {
            return match self.loops.last() {
                Some(Target::Loop(_)) => Ok(self.loops.len() - 1),
                Some(Target::Closure) => Err(closure()),
                None => Err(Error::ty(span, format!("`{what}` outside of a loop"))),
            };
        };
        let named = |t: &Target| match t {
            Target::Loop(lp) => lp.label.as_ref().is_some_and(|l| l.name.name == label.name),
            Target::Closure => false,
        };
        let Some(i) = self.loops.iter().rposition(named) else {
            return Err(Error::ty(
                label.span,
                format!("no loop around this `{what}` is labelled `{}`", label.name),
            ));
        };
        if self.loops[i..].iter().any(|t| matches!(t, Target::Closure)) {
            return Err(closure());
        }
        if let Target::Loop(lp) = &mut self.loops[i] {
            lp.used = true;
        }
        Ok(i)
    }

    //   Γ ⊢ e1 : T1    Γ, x : T1 ⊢ rest : T
    //  ------------------------------------- (T-Let)
    //     Γ ⊢ { let x = e1; rest } : T
    //
    // `Γ, x : T1` is "Γ extended with x" (shadowing any earlier x).
    // Expression statements `e;` are typed and their value discarded. An
    // `if`/`match`/block statement without the `;` must be `()`.
    //
    // With a pattern, `let p = e1;` extends Γ with every variable `p` binds.
    // `p` must be *irrefutable* (match every value of type T1): there's no
    // other branch to take if it doesn't.
    //
    // `want` is the expected type of the result, when checking (see `check`).
    //
    // A nested `fn` extends the context of the whole block, before it as
    // well as after, so that it can be recursive, and nested functions can
    // call each other. Its body is checked as if it were at the top level.
    //
    //   Γ ⊢ { stmts } diverges
    //  ------------------------ (T-Block-Never)
    //    Γ ⊢ { stmts } : !
    //
    // A block with no tail is a `()`, unless it never gets to its end, say
    // as it ends with a `return`: then it's a `!`. Code that comes after
    // something that diverges never runs, and gets a warning.
    fn block(
        &mut self,
        stmts: &[Stmt],
        tail: Option<&Expr>,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let scope = self.locals.len();
        let fns: Vec<&FnDecl> = stmts.iter().filter_map(Stmt::as_fn).collect();
        for (i, f) in fns.iter().enumerate() {
            if fns[..i].iter().any(|g| g.name.name == f.name.name) {
                return Err(Error::ty(
                    f.name.span,
                    format!("`{}` is already defined", f.name.name),
                ));
            }
        }
        if !fns.is_empty() {
            let fns = fns.iter().map(|f| self.nested_ids[&f.name.id]).collect();
            self.scopes.push(FnScope { at: scope, fns });
        }
        // If the code before the block diverges, that's for the code
        // around it to warn about.
        let mut warned = self.diverges.is_some();
        for (k, stmt) in stmts.iter().enumerate() {
            if !warned
                && self.diverges.is_some()
                && let Some(span) = stmt_span(stmt)
            {
                self.warnings.push(unreachable_code(span));
                warned = true;
            }
            match stmt {
                Stmt::Fn(f) => {
                    let fix = to_closure(f, stmts, k, tail);
                    self.nested(f, fix)?;
                }
                // Checked with the others, before any function.
                Stmt::Type(_) => {}
                Stmt::Let { pat, ty, init, .. } => {
                    // `let x;`: given its value later, by assignment, which
                    // can only give a whole variable one.
                    let deferred = match (&pat.kind, init) {
                        (_, Some(_)) => None,
                        (PatternKind::Bind(name, _), None) => Some(name),
                        (_, None) => {
                            return Err(Error::ty(
                                pat.span,
                                "a `let` with no value can only declare a variable, like \
                                 `let x;`, as assignment gives a variable its value",
                            ));
                        }
                    };
                    let t = match (ty, init) {
                        (Some(ann), init) => {
                            let t = self.resolve(ann)?;
                            self.mention(&t);
                            if let Some(init) = init {
                                self.check(init, &t)?;
                            }
                            t
                        }
                        (None, Some(init)) => self.synth(init)?,
                        (None, None) => {
                            let name = deferred.expect("a variable");
                            self.fresh(
                                pat.span,
                                format!("the type of `{name}`"),
                                format!("annotate it, e.g. `let {name}: int;`"),
                            )
                        }
                    };
                    let mut binds = Vec::new();
                    let p = self.pattern(pat, &t, None, &mut binds)?;
                    self.note_binds(&binds, ty.is_none());
                    binds.iter().for_each(|l| self.bind(l));
                    let t = self.solved(&t);
                    if let (PatternKind::Bind(name, _), Some(init)) = (&pat.kind, init)
                        && name == modules::FOR_ITERATOR
                    {
                        self.for_iterator(&t, init.span)?;
                    }
                    // Only alternatives can be unreachable here. A whole `let`
                    // pattern only is when the value can't exist, like a
                    // `Never` parameter, and then nothing is wrong.
                    let p = [p];
                    let pats = std::slice::from_ref(pat);
                    for r in exhaust::redundant(&self.adts, &t, &p) {
                        match r {
                            Redundant::Alt(span) => self.warnings.push(unreachable_alt(span, pats)),
                            Redundant::Trim(span, lo, hi) => {
                                self.warnings.push(narrowable(span, lo, hi, pats));
                            }
                            Redundant::Arm(_) => {}
                        }
                    }
                    if let Some(missing) = exhaust::missing(&self.adts, &t, &p) {
                        return Err(Error::ty(
                            pat.span,
                            format!(
                                "refutable pattern in `let`: `{missing}` not covered; use `match` instead"
                            ),
                        ));
                    }
                    self.locals.extend(binds);
                }
                Stmt::Expr(e) => {
                    self.synth(e)?;
                }
                // Rust's rule, and the reason for it: in
                // `if c { 10 } else { 20 } - x` the `if` is its own statement,
                // and its value would silently be thrown away.
                Stmt::BlockLike(e) => {
                    let t = self.synth(e)?;
                    let what = match e.kind {
                        ExprKind::If(..) => "`if`",
                        ExprKind::Match(..) => "`match`",
                        ExprKind::Loop(..) => "`loop`",
                        _ => "block",
                    };
                    let (start, end) = (e.span.start, e.span.end);
                    self.fits(&t, &Type::Unit, result_span(e), |t, _| {
                        format!(
                            "expected `()`, found `{t}`: this {what} is a statement, so its \
                             value would be lost; add `;` to discard it, or wrap it in \
                             parentheses to use it in an expression"
                        )
                    })
                    .map_err(|err| {
                        err.with_fix(Fix::new(
                            "add `;` to discard the value",
                            vec![Edit::insert(end, ";")],
                        ))
                        .with_fix(Fix::new(
                            "wrap it in parentheses",
                            vec![Edit::insert(start, "("), Edit::insert(end, ")")],
                        ))
                    })?;
                }
            }
        }
        if let (false, Some(_), Some(tail)) = (warned, self.diverges, tail) {
            self.warnings.push(unreachable_code(tail.span));
        }
        let t = match (tail, want) {
            (Some(tail), Some(want)) => {
                self.check(tail, want)?;
                want.clone()
            }
            (Some(tail), None) => self.synth(tail)?,
            (None, _) if self.diverges.is_some() => Type::Never,
            (None, _) => Type::Unit,
        };
        self.end_scope(scope);
        if !fns.is_empty() {
            self.scopes.pop();
        }
        Ok(t)
    }

    /// Checks the iterator of a `for` loop, of type `t`, written at `span`,
    /// can be thrown away when the loop ends, as it is: the same check as
    /// for any variable, which borrowck makes too, but with a message that
    /// says what to do about it. If `t` isn't known yet, or isn't an
    /// iterator at all, that's for those to say.
    fn for_iterator(&self, t: &Type, span: Span) -> Result<(), Error> {
        if unify::has_vars(t) || self.has_unsettled_closure(t) {
            return Ok(());
        }
        let (env, eqs) = (&self.env, &self.adts.proj.local);
        let implements = |tr: &str| self.traits.implements(t, tr, env, eqs).is_ok();
        if !implements(ITERATOR) || implements("Discard") {
            return Ok(());
        }
        Err(Error::ty(
            span,
            format!(
                "a `for` throws its iterator away when it ends, and `{t}` isn't `Discard`; to \
                 keep it, iterate over a mutable reference to it: `for x in &mut it`"
            ),
        ))
    }
}

/// What a `break` or `continue` goes to.
#[derive(Debug)]
pub(crate) enum Target {
    Loop(Box<LoopCx>),
    /// The start of a closure's body, which is a function of its own: no
    /// `break` gets out of it.
    Closure,
}

/// A loop being checked.
#[derive(Debug)]
pub(crate) struct LoopCx {
    label: Option<Label>,
    /// Whether a `break` or `continue` has used its label.
    used: bool,
    /// A `while`, whose `break` can't give a value, rather than a `loop`.
    is_while: bool,
    /// The type of the loop's value wanted from it, when checking.
    want: Option<Type>,
    /// The type of the values its `break`s give it, once one is seen. It
    /// stays `None` if none ever gets out, so then the loop never ends, and
    /// is a `!`.
    value: Option<Type>,
}

/// Where a statement is, if it's code, which runs, rather than a nested
/// function or type.
pub(super) fn stmt_span(stmt: &Stmt) -> Option<Span> {
    match stmt {
        Stmt::Let { span, .. } => Some(*span),
        Stmt::Expr(e) | Stmt::BlockLike(e) => Some(e.span),
        Stmt::Fn(_) | Stmt::Type(_) => None,
    }
}

/// Where variable `name` is first used in `e`, if it is.
pub(crate) fn first_use(e: &Expr, name: &str) -> Option<Span> {
    match &e.kind {
        ExprKind::Var(n, _) if n == name => Some(e.span),
        _ => e.children().into_iter().find_map(|c| first_use(c, name)),
    }
}

/// Where to point when an expression has the wrong type: for a block that's
/// its final expression, not the whole multi-line block.
pub(crate) fn result_span(e: &Expr) -> Span {
    match &e.kind {
        ExprKind::Block(_, Some(tail)) => result_span(tail),
        _ => e.span,
    }
}
