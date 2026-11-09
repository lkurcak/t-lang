//! Refinements of type arguments of any type (docs/60-refined-type-arguments.md):
//! `List<{ t: usize | t < n }>` is a list each of whose elements is below
//! `n`.
//!
//! What's in a list isn't in the logic, as an enum's fields are (docs/53):
//! so such a refinement is *carried*, a fact of every value at a
//! *position* in the value's type (`Pos`), which the walk keeps of each
//! object (`Walk::trees`) and of each place, as what's written there must
//! keep (`Walk::place_decl`). Code generic in a type parameter `X` can't
//! make a value of it when only markers and *consumers* bound it (traits
//! whose methods take values and give back none, `consumer`), so, by
//! parametricity, at a call it can only give back, and write, values of
//! `X` it was given (lemma A2). Each call picks what `X` is, its
//! *instantiation* (`Walk::instantiate`): one of the values of it the
//! arguments have, each as known as it is there. What's behind a `&mut`
//! argument it's given is what the place there may hold (its declared
//! refinements), which what it writes there must keep: so the
//! instantiation must imply them (lemma A3). Where a refinement is checked
//! (a `let`, an argument, a result, a field), each of its positions is
//! checked as borrowck's variances say (lemma A4): where nothing can write
//! through an alias (covariant), what the value has must imply it; where
//! something can (invariant: behind a `&mut`, in an `Unsafe`), what's
//! declared there must be exactly it.

use super::*;
use crate::borrowck::regions::Variance;
use std::rc::Rc;

/// A position in a type: which of the types in it a refinement can be in
/// (`TypeExpr::refinable`), from the top, at each depth: a type argument
/// (not counting lifetimes), a tuple's element, what a reference or a box
/// points to, a slice's or an array's elements. `[]` is the value itself;
/// `[0, 1]` is each `u8` in a `List<(u64, u8)>`.
pub(super) type Pos = Vec<usize>;

/// A refinement's predicate, its names other than its binder bound to the
/// values they had where it was said (lemma A1): a fixed predicate of
/// values, as objects never change.
#[derive(Debug)]
pub(super) struct Closed {
    pred: Expr,
    names: HashMap<NodeId, Val>,
}

/// What a value is: one value; one a predicate is true of; one each of
/// some `Disj`s is true of; or one nothing's known of.
#[derive(Debug, Clone)]
pub(super) enum Cand {
    Exact(Val),
    Pred(Rc<Closed>),
    All(Vec<Disj>),
    Any,
}

/// What each value at a position is: one of the cases, each where its
/// conditions hold. With none, there are no values there.
#[derive(Debug, Clone, Default)]
pub(super) struct Disj(pub(super) Vec<(Vec<Formula>, Cand)>);

impl Disj {
    fn any() -> Disj {
        Disj(vec![(Vec::new(), Cand::Any)])
    }

    fn one(c: Cand) -> Disj {
        Disj(vec![(Vec::new(), c)])
    }

    /// Does it say nothing, so that any value has it?
    fn is_any(&self) -> bool {
        self.0.iter().any(|(g, c)| g.is_empty() && c.is_any())
    }

    /// It, where `g` holds: of what's there if `g` doesn't, nothing is
    /// said.
    fn under(&self, g: &Formula) -> Disj {
        let mut d = self.clone();
        d.0.push((vec![g.clone().not()], Cand::Any));
        d
    }

    /// A predicate it says, for messages.
    fn text(&self) -> Option<String> {
        self.0.iter().find_map(|(_, c)| c.text())
    }
}

impl Cand {
    fn is_any(&self) -> bool {
        match self {
            Cand::Any => true,
            Cand::All(ds) => ds.iter().all(Disj::is_any),
            _ => false,
        }
    }

    fn text(&self) -> Option<String> {
        match self {
            Cand::Pred(c) => Some(crate::pretty::expr_text(&c.pred)),
            Cand::All(ds) => ds.iter().find_map(Disj::text),
            _ => None,
        }
    }
}

/// What's said of each position of a value: all of it holds.
pub(super) type Tree = Vec<(Pos, Disj)>;

/// How deep a position is followed: deeper than any type written by hand.
const DEPTH: usize = 6;

/// Where a candidate of an instantiation comes from, for messages: which
/// argument, and whether it's what a `&mut` argument's place may hold.
#[derive(Debug, Clone)]
struct Source {
    arg: usize,
    behind: bool,
}

/// What a call's type parameter `X` is (`Walk::instantiate`): at each
/// position of the type it's given, the cases, each from an argument, and
/// why it may be anything, if it may.
#[derive(Debug, Clone, Default)]
pub(super) struct Inst {
    at: Vec<(Pos, Vec<(Source, Vec<Formula>, Cand)>)>,
    /// Why nothing's known of it: a bound that can make one.
    any: Option<String>,
}

impl Inst {
    /// What it says of position `pos`.
    fn disj(&self, pos: &[usize]) -> Disj {
        if self.any.is_some() {
            return Disj::any();
        }
        match self.at.iter().find(|(p, _)| p == pos) {
            Some((_, cases)) => Disj(cases.iter().map(|(_, g, c)| (g.clone(), c.clone())).collect()),
            None => Disj::default(),
        }
    }

    /// It as a tree, of the type it's given.
    fn tree(&self) -> Tree {
        if self.any.is_some() {
            return Vec::new();
        }
        self.at.iter().map(|(p, _)| (p.clone(), self.disj(p))).collect()
    }
}

/// A check a call needs of what's behind a `&mut` (or in an `Unsafe`) an
/// argument gives it: that what the callee may write there, `inst` at each
/// position below `at` in the parameter, has what the place must keep.
struct Behind {
    arg: usize,
    /// The position in the argument.
    at: Pos,
    /// What's declared there, at each position below `at`.
    declared: Tree,
}

/// Is trait `name` one whose values only ever go in, never out: a marker,
/// or a trait each of whose methods takes `Self` only as `self`, `&self` or
/// a `&Self`, gives back nothing with `Self` in it, has no associated type,
/// and whose supertraits are such too? Its methods can make no value of
/// `Self`, nor change one (lemma A5). Not `Int`, whose literals make values,
/// nor `Atom`, which `Volatile::at` reads from an address.
pub(super) fn consumer(traits: &crate::traits::Traits, name: &str, seen: &mut Vec<String>) -> bool {
    use crate::traits::{COPY, DISCARD, FN_TRAITS, INT, SEND, SIGNED, SYNC, UNSIGNED};
    if [COPY, DISCARD, SEND, SYNC].contains(&name) {
        return true;
    }
    if [INT, SIGNED, UNSIGNED, "core::sync::Atom"].contains(&name) || FN_TRAITS.contains(&name) {
        return false;
    }
    if seen.iter().any(|s| s == name) {
        return true;
    }
    seen.push(name.to_string());
    let Some(def) = traits.traits.get(name) else {
        return false;
    };
    if def.assoc.iter().any(|a| !a.effect) {
        return false;
    }
    if !def.supers.iter().all(|s| consumer(traits, s, seen)) {
        return false;
    }
    def.methods.iter().all(|m| {
        let Type::Fn(sig) = &m.scheme.ty else {
            return false;
        };
        let this = |t: &Type| matches!(t, Type::Param(p, _) if p == "Self");
        let mentions = |t: &Type| free::mentions(t, "Self");
        let params_ok = sig.params.iter().all(|p| match p {
            t if this(t) => true,
            Type::Ref(_, false, inner) if this(inner) => true,
            t => !mentions(t),
        });
        let preds_ok = m.scheme.preds.iter().all(|p| match this(&p.ty) {
            true => p.trait_name == name || consumer(traits, &p.trait_name, seen),
            false => !mentions(&p.ty),
        });
        let eqs_ok = (m.scheme.eqs.iter()).all(|(a, b)| !mentions(a) && !mentions(b));
        params_ok && preds_ok && eqs_ok && !mentions(&sig.ret)
    })
}

impl Walk<'_> {
    /// How struct or enum `symbol` varies with each of its lifetime
    /// parameters, then each of its type parameters, as the borrow checker
    /// says (`borrowck::regions::AdtInfo`).
    fn variances(&mut self, symbol: &str) -> Vec<Variance> {
        let info = self.cx.adt_info();
        info.variances(symbol).map(<[Variance]>::to_vec).unwrap_or_default()
    }

    /// The type at child `k` of `t` (a position's step), and how `t`
    /// varies with it.
    pub(super) fn child(&mut self, t: &Type, k: usize) -> Option<(Type, Variance)> {
        match t {
            Type::Adt(symbol, lts, args) => {
                let arg = self.cx.solved(args.get(k)?);
                let v = self.variances(symbol).get(lts.len() + k).copied();
                Some((arg, v.unwrap_or(Variance::Inv)))
            }
            Type::Box(inner) if k == 0 => Some((self.cx.solved(inner), Variance::Co)),
            Type::Ref(_, m, inner) if k == 0 => Some((
                self.cx.solved(inner),
                if *m { Variance::Inv } else { Variance::Co },
            )),
            Type::Slice(inner) | Type::Array(inner, _) if k == 0 => {
                Some((self.cx.solved(inner), Variance::Co))
            }
            Type::Tuple(ts) => Some((self.cx.solved(ts.get(k)?), Variance::Co)),
            _ => None,
        }
    }

    /// How many children `t` has.
    fn arity(t: &Type) -> usize {
        match t {
            Type::Adt(_, _, args) => args.len(),
            Type::Box(_) | Type::Ref(..) | Type::Slice(_) | Type::Array(..) => 1,
            Type::Tuple(ts) => ts.len(),
            _ => 0,
        }
    }

    /// The type at position `pos` of `t`, and how `t` varies with it.
    pub(super) fn type_at(&mut self, t: &Type, pos: &[usize]) -> Option<(Type, Variance)> {
        let mut ty = t.clone();
        let mut v = Variance::Co;
        for &k in pos {
            let (next, w) = self.child(&ty, k)?;
            v = v.then(w);
            ty = next;
        }
        Some((ty, v))
    }

    /// Every position of `t` but the whole value's, as deep as `DEPTH`.
    fn positions(&mut self, t: &Type) -> Vec<Pos> {
        fn go(w: &mut Walk, t: &Type, at: &mut Pos, out: &mut Vec<Pos>) {
            if at.len() >= DEPTH {
                return;
            }
            for k in 0..Walk::arity(t) {
                let Some((c, _)) = w.child(t, k) else {
                    continue;
                };
                at.push(k);
                out.push(at.clone());
                go(w, &c, at, out);
                at.pop();
            }
        }
        let mut out = Vec::new();
        go(self, t, &mut Vec::new(), &mut out);
        out
    }

    /// The positions of type parameter `x` in `t`, a type as declared, with
    /// how `t` varies with each; and whether it's anywhere else in it, in a
    /// function's type, a projection or an `impl` type, where nothing's
    /// followed, and how (`Some(true)` if as a result, which is where a
    /// value of it could come from).
    fn occurrences(&mut self, t: &Type, x: &str) -> (Vec<(Pos, Variance)>, bool) {
        fn go(
            w: &mut Walk,
            t: &Type,
            x: &str,
            at: &mut Pos,
            v: Variance,
            out: &mut Vec<(Pos, Variance)>,
            hidden: &mut bool,
        ) {
            if matches!(t, Type::Param(p, _) if p == x) {
                out.push((at.clone(), v));
                return;
            }
            if !free::mentions(t, x) {
                return;
            }
            let n = Walk::arity(t);
            if n == 0 || at.len() >= DEPTH {
                *hidden = true;
                return;
            }
            for k in 0..n {
                let Some((c, cv)) = w.child_declared(t, k) else {
                    *hidden = true;
                    continue;
                };
                at.push(k);
                go(w, &c, x, at, v.then(cv), out, hidden);
                at.pop();
            }
        }
        let mut out = Vec::new();
        let mut hidden = false;
        go(self, t, x, &mut Vec::new(), Variance::Co, &mut out, &mut hidden);
        (out, hidden)
    }

    /// `child`, of a type as declared, not solved: its type parameters
    /// stay.
    fn child_declared(&mut self, t: &Type, k: usize) -> Option<(Type, Variance)> {
        match t {
            Type::Adt(symbol, lts, args) => {
                let arg = args.get(k)?.clone();
                let v = self.variances(symbol).get(lts.len() + k).copied();
                Some((arg, v.unwrap_or(Variance::Inv)))
            }
            Type::Box(inner) if k == 0 => Some(((**inner).clone(), Variance::Co)),
            Type::Ref(_, m, inner) if k == 0 => Some((
                (**inner).clone(),
                if *m { Variance::Inv } else { Variance::Co },
            )),
            Type::Slice(inner) | Type::Array(inner, _) if k == 0 => {
                Some(((**inner).clone(), Variance::Co))
            }
            Type::Tuple(ts) => Some((ts.get(k)?.clone(), Variance::Co)),
            _ => None,
        }
    }

    /// For each field of struct or enum `t`'s value whose type has its
    /// type parameter `k` in it, as each variant's field, `(variant, field,
    /// position of the parameter in the field's type, the field's type)`.
    /// A tuple's element `k` is its field `k`, whole. Nothing of an
    /// `Unsafe`, whose value can change through a `&` (docs/46): what's in
    /// one is only what its refinements say.
    fn fields_with(&mut self, t: &Type, k: usize) -> Vec<(usize, usize, Pos, Type)> {
        match t {
            Type::Tuple(ts) => match ts.get(k) {
                Some(e) => vec![(0, k, Vec::new(), self.cx.solved(e))],
                None => Vec::new(),
            },
            Type::Adt(symbol, ..)
                if symbol == crate::check::UNSAFE || crate::intrinsic::is_intrinsic_type(symbol) =>
            {
                Vec::new()
            }
            Type::Adt(symbol, ..) => {
                let Some(def) = self.cx.adts.get(symbol).cloned() else {
                    return Vec::new();
                };
                let Some(param) = def.params.as_slice().get(k).cloned() else {
                    return Vec::new();
                };
                let variants: Vec<Vec<Type>> = match &def.kind {
                    crate::types::AdtKind::Struct(fields) => {
                        vec![fields.iter().map(|(_, t)| t.clone()).collect()]
                    }
                    crate::types::AdtKind::Enum(vs) => vs.iter().map(|(_, ts)| ts.clone()).collect(),
                };
                let mut out = Vec::new();
                for (v, fields) in variants.iter().enumerate() {
                    let insts: Vec<Type> = match &def.kind {
                        crate::types::AdtKind::Struct(_) => (struct_fields(&self.cx.adts, t))
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(_, t)| t)
                            .collect(),
                        crate::types::AdtKind::Enum(_) => {
                            crate::types::variant_fields(&self.cx.adts, t, v)
                        }
                    };
                    for (f, ft) in fields.iter().enumerate() {
                        let (occ, _) = self.occurrences(ft, &param);
                        for (rho, _) in occ {
                            let inst = insts.get(f).map(|t| self.cx.solved(t));
                            out.push((v, f, rho, inst.unwrap_or(Type::Unit)));
                        }
                    }
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// The object value `v` is, if it has one.
    fn obj_of(v: &Val) -> Option<Obj> {
        match v {
            Val::Struct(o, _) | Val::Ref(o, _) | Val::Obj(o) | Val::Array(o, _) => Some(*o),
            _ => None,
        }
    }

    /// What's been said of the values at position `pos` of object `o`.
    fn registered(&self, o: Option<Obj>, pos: &[usize]) -> Vec<Disj> {
        let Some(o) = o else {
            return Vec::new();
        };
        (self.trees.get(&o).into_iter().flatten())
            .filter(|(p, _)| p == pos)
            .map(|(_, d)| d.clone())
            .collect()
    }

    /// What's known of the values at position `pos` of `v`, of type `t`:
    /// cases that are all of them, each where its conditions hold (lemma
    /// A6): of the value itself, it; of what's in its fields, the walk
    /// knows (a tuple's or a struct's, an enum's of the variant it is), and
    /// of what a reference points to, what's there now; and what's been said
    /// of the positions of each object on the way.
    pub(super) fn values_at(&mut self, v: &Val, t: &Type, pos: &[usize]) -> Vec<(Vec<Formula>, Cand)> {
        self.values_at_depth(v, t, pos, 0)
    }

    fn values_at_depth(
        &mut self,
        v: &Val,
        t: &Type,
        pos: &[usize],
        depth: usize,
    ) -> Vec<(Vec<Formula>, Cand)> {
        let reg = self.registered(Self::obj_of(v), pos);
        let with = |cases: Vec<(Vec<Formula>, Cand)>| -> Vec<(Vec<Formula>, Cand)> {
            if reg.is_empty() {
                return cases;
            }
            (cases.into_iter())
                .map(|(g, c)| {
                    let mut all = reg.clone();
                    all.push(Disj::one(c));
                    (g, Cand::All(all))
                })
                .collect()
        };
        let Some((&k, rest)) = pos.split_first() else {
            return match v {
                Val::Other => with(vec![(Vec::new(), Cand::Any)]),
                v => with(vec![(Vec::new(), Cand::Exact(v.clone()))]),
            };
        };
        if depth > DEPTH {
            return with(vec![(Vec::new(), Cand::Any)]);
        }
        let mut cases = Vec::new();
        let opaque = match (v, t) {
            (Val::Struct(_, vals), Type::Tuple(ts)) => {
                match (vals.get(k), ts.get(k)) {
                    (Some(e), Some(et)) => {
                        let et = self.cx.solved(et);
                        cases = self.values_at_depth(e, &et, rest, depth + 1);
                    }
                    _ => cases.push((Vec::new(), Cand::Any)),
                }
                false
            }
            (Val::Struct(_, vals), Type::Adt(symbol, ..)) if !opaque_type(symbol) => {
                let vals = vals.clone();
                for (_, f, rho, ft) in self.fields_with(t, k) {
                    let Some(fv) = vals.get(f) else {
                        cases.push((Vec::new(), Cand::Any));
                        continue;
                    };
                    let mut at = rho;
                    at.extend_from_slice(rest);
                    cases.extend(self.values_at_depth(fv, &ft, &at, depth + 1));
                }
                false
            }
            (Val::Obj(o), Type::Adt(symbol, ..))
                if self.parts.contains_key(o) && !opaque_type(symbol) =>
            {
                let parts = self.parts[o].clone();
                for (variant, f, rho, ft) in self.fields_with(t, k) {
                    let Some(fv) = parts.fields.get(variant).and_then(|fs| fs.get(f)).cloned()
                    else {
                        cases.push((Vec::new(), Cand::Any));
                        continue;
                    };
                    let n = i128::try_from(variant).expect("few variants");
                    let Some(tag) = Formula::eq(&parts.tag, &Lin::constant(n)) else {
                        cases.push((Vec::new(), Cand::Any));
                        continue;
                    };
                    let mut at = rho;
                    at.extend_from_slice(rest);
                    for (mut g, c) in self.values_at_depth(&fv, &ft, &at, depth + 1) {
                        g.insert(0, tag.clone());
                        cases.push((g, c));
                    }
                }
                false
            }
            (Val::Ref(_, Pointee::Val(w)), Type::Ref(_, _, inner) | Type::Box(inner)) if k == 0 => {
                let (w, inner) = ((**w).clone(), self.cx.solved(inner));
                cases = self.values_at_depth(&w, &inner, rest, depth + 1);
                false
            }
            (Val::Ref(_, Pointee::Place(p)), Type::Ref(_, _, inner) | Type::Box(inner))
                if k == 0 =>
            {
                let inner = self.cx.solved(inner);
                let w = self.read_place(&p.clone());
                cases = self.values_at_depth(&w, &inner, rest, depth + 1);
                false
            }
            _ => true,
        };
        if opaque {
            return match reg.is_empty() {
                true => vec![(Vec::new(), Cand::Any)],
                false => vec![(Vec::new(), Cand::All(reg))],
            };
        }
        with(cases)
    }

    /// Says `entries`, a tree of a value of type `t`, of `v`: of each of its
    /// objects, and of what's in its fields the walk knows, of each variant
    /// of an enum's where it's that variant; of the value itself, as a fact.
    pub(super) fn assume_tree(&mut self, v: &Val, t: &Type, entries: &Tree) {
        self.assume_tree_depth(v, t, entries, 0)
    }

    fn assume_tree_depth(&mut self, v: &Val, t: &Type, entries: &Tree, depth: usize) {
        if entries.is_empty() || depth > DEPTH {
            return;
        }
        for (pos, d) in entries {
            if d.is_any() {
                continue;
            }
            if pos.is_empty() {
                if let Some(f) = self.disj_formula(d, v, false) {
                    self.fact(f);
                }
            }
            if let Some(o) = Self::obj_of(v) {
                let tree = self.trees.entry(o).or_default();
                if !tree.iter().any(|(p, e)| p == pos && std::ptr::eq(e, d)) {
                    tree.push((pos.clone(), d.clone()));
                }
            }
            let Some((&k, rest)) = pos.split_first() else {
                continue;
            };
            match (v, t) {
                (Val::Struct(_, vals), Type::Tuple(ts)) => {
                    if let (Some(e), Some(et)) = (vals.get(k).cloned(), ts.get(k)) {
                        let et = self.cx.solved(et);
                        let sub = vec![(rest.to_vec(), d.clone())];
                        self.assume_tree_depth(&e, &et, &sub, depth + 1);
                    }
                }
                (Val::Struct(_, vals), Type::Adt(symbol, ..)) if !opaque_type(symbol) => {
                    let vals = vals.clone();
                    for (_, f, rho, ft) in self.fields_with(t, k) {
                        if let Some(fv) = vals.get(f) {
                            let mut at = rho;
                            at.extend_from_slice(rest);
                            self.assume_tree_depth(fv, &ft, &vec![(at, d.clone())], depth + 1);
                        }
                    }
                }
                (Val::Obj(o), Type::Adt(..)) if self.parts.contains_key(o) => {
                    let parts = self.parts[o].clone();
                    for (variant, f, rho, ft) in self.fields_with(t, k) {
                        let Some(fv) = parts.fields.get(variant).and_then(|fs| fs.get(f)).cloned()
                        else {
                            continue;
                        };
                        let n = i128::try_from(variant).expect("few variants");
                        let Some(tag) = Formula::eq(&parts.tag, &Lin::constant(n)) else {
                            continue;
                        };
                        let mut at = rho;
                        at.extend_from_slice(rest);
                        let sub = vec![(at, d.under(&tag))];
                        self.assume_tree_depth(&fv, &ft, &sub, depth + 1);
                    }
                }
                (Val::Ref(_, Pointee::Val(w)), Type::Ref(_, _, inner) | Type::Box(inner))
                    if k == 0 =>
                {
                    let (w, inner) = ((**w).clone(), self.cx.solved(inner));
                    self.assume_tree_depth(&w, &inner, &vec![(rest.to_vec(), d.clone())], depth + 1);
                }
                _ => {}
            }
        }
    }

    /// That `v` has `d`, if the logic can say: of a case it can't say,
    /// nothing, unless `strict`, when it can't say it at all.
    pub(super) fn disj_formula(&mut self, d: &Disj, v: &Val, strict: bool) -> Option<Formula> {
        let mut cases = Vec::new();
        for (g, c) in &d.0 {
            let f = self.cand_formula(c, v, strict)?;
            let mut all = g.clone();
            all.push(f);
            cases.push(Formula::and(all));
        }
        Some(Formula::or(cases))
    }

    fn cand_formula(&mut self, c: &Cand, v: &Val, strict: bool) -> Option<Formula> {
        match c {
            Cand::Any => Some(Formula::True),
            Cand::Exact(w) => {
                let same = self.same(v, w);
                match same.is_empty() {
                    true if strict => None,
                    true => Some(Formula::True),
                    false => Some(Formula::and(same)),
                }
            }
            Cand::Pred(closed) => {
                let closed = closed.clone();
                match self.term(&closed.pred, v, Env::Closed(&closed.names)) {
                    Ok(Val::Bool(f)) => Some(f),
                    _ if strict => None,
                    _ => Some(Formula::True),
                }
            }
            Cand::All(ds) => {
                let mut all = Vec::new();
                for d in ds {
                    match self.disj_formula(d, v, strict) {
                        Some(f) => all.push(f),
                        None if strict => return None,
                        None => {}
                    }
                }
                Some(Formula::and(all))
            }
        }
    }

    /// Does every value one of `cases` (at a position of type `t`) is have
    /// `goal`? Each case is checked as a value of its own: one nothing's
    /// known of but what the case says, or the value it is.
    pub(super) fn implies(
        &mut self,
        cases: &[(Vec<Formula>, Cand)],
        goal: &Disj,
        t: &Type,
    ) -> Result<(), Failure> {
        if goal.is_any() || !self.reachable {
            return Ok(());
        }
        for (g, c) in cases {
            let start = self.facts.len();
            for f in g {
                self.fact(f.clone());
            }
            let x = match c {
                Cand::Exact(w) => w.clone(),
                c => {
                    let x = self.unknown(t);
                    if let Some(f) = self.cand_formula(c, &x, false) {
                        self.fact(f);
                    }
                    x
                }
            };
            let outcome = match self.disj_formula(goal, &x, true) {
                Some(f) => self.proves(Some(&f)),
                None => Err(Failure::Unsupported),
            };
            self.facts.truncate(start);
            outcome?;
        }
        Ok(())
    }

    /// Refinement part `part`, of a type argument, its names other than its
    /// binder bound to their values as `env` says now (lemma A1), if the
    /// logic can say them.
    pub(super) fn close(&mut self, pred: &Expr, env: Env) -> Option<Rc<Closed>> {
        let mut names = HashMap::new();
        self.close_names(pred, env, &mut names).ok()?;
        Some(Rc::new(Closed {
            pred: pred.clone(),
            names,
        }))
    }

    fn close_names(
        &mut self,
        e: &Expr,
        env: Env,
        out: &mut HashMap<NodeId, Val>,
    ) -> Result<(), Failure> {
        match &e.kind {
            ExprKind::Var(..) => {
                if let Some(name) = self.cx.pred_names.get(&e.id)
                    && !matches!(name, PredName::Binder)
                {
                    let v = self.term(e, &Val::Other, env)?;
                    out.insert(e.id, v);
                }
                Ok(())
            }
            ExprKind::Call(_, args) if args.len() == 1 => match env {
                Env::Post(old, _, targs) => self.close_names(&args[0], Env::Call(old, targs), out),
                Env::Closed(_) => self.close_names(&args[0], env, out),
                _ => Err(Failure::Unsupported),
            },
            _ => {
                for c in e.children() {
                    self.close_names(c, env, out)?;
                }
                Ok(())
            }
        }
    }

    /// The tree refinement `r`'s carried parts say, their names as `env`
    /// says now.
    pub(super) fn carried_tree(&mut self, r: &Refinement, env: Env) -> Tree {
        let mut out = Vec::new();
        for part in r.parts.iter().filter(|p| p.carried) {
            let d = match self.close(&part.pred, env) {
                Some(c) => Disj::one(Cand::Pred(c)),
                None => continue,
            };
            out.push((part.path.clone(), d));
        }
        out
    }

    /// Checks that `v`, of type `t`, has the carried parts of `r` (names as
    /// `env` says): at each, each value there has it; and where `t` is
    /// invariant there, what may be written there (`decl`, what's declared
    /// of `v`'s places, if it's a reference) is exactly it, both ways.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn need_carried(
        &mut self,
        r: &Refinement,
        v: &Val,
        t: &Type,
        env: Env,
        decl: Option<&Tree>,
        span: Span,
        what: &str,
    ) -> Result<(), Error> {
        let tree = self.carried_tree(r, env);
        let missing = r.parts.iter().filter(|p| p.carried).count() != tree.len();
        if missing {
            let part = r.parts.iter().find(|p| p.carried).expect("carried");
            return Err(Error::ty(
                span,
                format!("can't prove `{}` for {what}", part.text()),
            ));
        }
        self.need_tree(v, t, &tree, decl, span, what)
    }

    /// Checks that `v`, of type `t`, has `tree` (see `need_carried`).
    pub(super) fn need_tree(
        &mut self,
        v: &Val,
        t: &Type,
        tree: &Tree,
        decl: Option<&Tree>,
        span: Span,
        what: &str,
    ) -> Result<(), Error> {
        for (pos, d) in tree {
            let Some((pt, var)) = self.type_at(t, pos) else {
                continue;
            };
            let cases = self.values_at(v, t, pos);
            if let Err(reason) = self.implies(&cases, d, &pt) {
                let text = d.text().unwrap_or_default();
                let failure = self.proof_failure(reason, None, None, &text);
                return Err(failure.attach(Error::ty(
                    span,
                    format!("can't prove `{text}` for {what}"),
                )));
            }
            if var == Variance::Inv {
                let declared: Vec<Disj> = (decl.into_iter().flatten())
                    .filter(|(p, _)| p == pos)
                    .map(|(_, d)| d.clone())
                    .collect();
                for want in declared {
                    let cases = d.0.clone();
                    if self.implies(&cases, &want, &pt).is_err() {
                        let text = want.text().unwrap_or_default();
                        return Err(Error::ty(
                            span,
                            format!("can't prove `{text}` of what may be written through {what}"),
                        ));
                    }
                }
            }
        }
        // Where it's invariant and says nothing, what's declared must say
        // nothing either.
        for (pos, want) in decl.into_iter().flatten() {
            if tree.iter().any(|(p, _)| p == pos) || want.is_any() {
                continue;
            }
            let Some((pt, Variance::Inv)) = self.type_at(t, pos) else {
                continue;
            };
            if self.implies(&[(Vec::new(), Cand::Any)], want, &pt).is_err() {
                let text = want.text().unwrap_or_default();
                return Err(Error::ty(
                    span,
                    format!("can't prove `{text}` of what may be written through {what}"),
                ));
            }
        }
        Ok(())
    }

    /// What's declared of the value at place `p`: what every write there
    /// must keep (lemma A6). Of a local, its refinements' carried parts, or
    /// for a `let` without one, what its value's type says (`widen`); of a
    /// field, what's declared of the struct, at the field's type's
    /// positions, and the field's own refinements', of the struct's fields'
    /// values now; behind a box, what's declared of it there; behind a
    /// reference, what the reference says, which is what its place had
    /// declared when it was made.
    pub(super) fn place_decl(&mut self, p: &Place) -> Tree {
        self.place_decl_depth(p, 0, true)
    }

    /// What's said of a value read at place `p` that's new: what's
    /// declared there, but for its own struct field refinements, which
    /// `assume_fields` says of all the fields' values (as reading them here
    /// would read this one again); and what the call that last lent it left
    /// there (`lent_trees`).
    pub(super) fn place_read(&mut self, p: &Place) -> Tree {
        let mut tree = self.place_decl_depth(p, 0, false);
        if let Some(t) = self.lent_trees.get(p) {
            tree.extend(t.iter().cloned());
        }
        tree
    }

    fn place_decl_depth(&mut self, p: &Place, depth: usize, fields: bool) -> Tree {
        if depth > 2 * DEPTH {
            return Vec::new();
        }
        let Some((last, init)) = p.path.split_last() else {
            return self.decl.get(&p.root).cloned().unwrap_or_default();
        };
        let q = Place {
            root: p.root,
            path: init.to_vec(),
        };
        let qt = self.place_ty(&q);
        let outer = self.place_decl_depth(&q, depth + 1, fields);
        match last {
            Proj::Deref => {
                let mut out = below(&outer, 0);
                if let Type::Ref(..) = qt {
                    // What the reference itself says.
                    if let Some(&o) = self.objs.get(&q) {
                        let reg = self.trees.get(&o).cloned().unwrap_or_default();
                        out.extend(below(&reg, 0));
                    }
                }
                out
            }
            Proj::Field(j) => {
                let mut out = Vec::new();
                for (pos, d) in &outer {
                    let Some((&k, rest)) = pos.split_first() else {
                        continue;
                    };
                    for (_, f, rho, _) in self.fields_with(&qt, k) {
                        if f == *j {
                            let mut at = rho;
                            at.extend_from_slice(rest);
                            out.push((at, d.clone()));
                        }
                    }
                }
                // The field's own refinements, of what it's in.
                if fields
                    && let Some(fields) = self.refined_fields(&qt).cloned()
                    && let Some(Some(r)) = fields.refs.get(*j)
                    && r.has_carried()
                {
                    let vals = self.fields_of(&q, &qt).unwrap_or_default();
                    let args = self.adt_args(&qt);
                    out.extend(self.carried_tree(r, Env::Fields(&vals, &args)));
                }
                out
            }
        }
    }
}

/// Is struct `symbol` one whose values' fields aren't what it holds: an
/// `Unsafe`, whose value can change through a `&` (docs/46), or an
/// intrinsic type, which has none? What's in one is only what's said of it.
fn opaque_type(symbol: &str) -> bool {
    symbol == crate::check::UNSAFE || crate::intrinsic::is_intrinsic_type(symbol)
}

/// What `tree` says below child `k` of the top, from there.
pub(super) fn below(tree: &Tree, k: usize) -> Tree {
    (tree.iter())
        .filter_map(|(p, d)| match p.split_first() {
            Some((&j, rest)) if j == k => Some((rest.to_vec(), d.clone())),
            _ => None,
        })
        .collect()
}

/// `tree` under child `k`: what it says, of child `k` of a value.
pub(super) fn under(tree: &Tree, k: usize) -> Tree {
    (tree.iter())
        .map(|(p, d)| {
            let mut at = vec![k];
            at.extend_from_slice(p);
            (at, d.clone())
        })
        .collect()
}

/// What a `let` without a refinement declares, of the value it's given
/// whose tree is `tree`: what refinements say, but not what's known only of
/// the values there now (which are what the program writes there next
/// replaces), as TypeScript widens a literal's type for a `let`. So `let
/// mut l = List::new();` declares nothing of its elements, though there are
/// none, and the iterator `for` makes has its list's elements' refinements.
pub(super) fn widen(tree: &Tree) -> Tree {
    fn cand(c: &Cand) -> Cand {
        match c {
            Cand::Exact(_) | Cand::Any => Cand::Any,
            Cand::Pred(p) => Cand::Pred(p.clone()),
            Cand::All(ds) => Cand::All(ds.iter().map(disj).collect()),
        }
    }
    fn disj(d: &Disj) -> Disj {
        if d.0.is_empty() {
            return Disj::any();
        }
        Disj(d.0.iter().map(|(g, c)| (g.clone(), cand(c))).collect())
    }
    (tree.iter())
        .map(|(p, d)| (p.clone(), disj(d)))
        .filter(|(_, d)| !d.is_any())
        .collect()
}

/// Where a type parameter's values come from that a call can't follow: a
/// bound on it that makes values (`consumer`), or another that might,
/// or a closure's result. `None` if there's none (lemma A2).
pub(super) fn makes(
    traits: &crate::traits::Traits,
    s: &crate::types::Scheme,
    x: &str,
) -> Option<String> {
    let this = s.params.ty(x);
    for p in &s.preds {
        if p.ty == this {
            if !consumer(traits, &p.trait_name, &mut Vec::new()) {
                let name = traits.shorten(&p.trait_name);
                return Some(format!("`{x}: {name}`"));
            }
        } else if free::mentions(&p.ty, x)
            && !consumer(traits, &p.trait_name, &mut Vec::new())
            && !crate::traits::FN_TRAITS.contains(&p.trait_name.as_str())
        {
            let name = traits.shorten(&p.trait_name);
            return Some(format!("`{}: {name}`", p.ty));
        }
    }
    for (a, b) in &s.eqs {
        // What an impl says its trait's associated type is, for its own
        // type, isn't a bound: a value of it is made only by the impl's
        // methods, which can make an `x` no more than this one can.
        if let Type::Proj(base, ..) = a
            && !matches!(**base, Type::Param(..))
            && !free::mentions(a, x)
        {
            continue;
        }
        if let Type::Proj(base, ..) = a
            && !matches!(**base, Type::Param(..) | Type::Proj(..))
        {
            continue;
        }
        if crate::traits::is_sig(a) {
            if let Type::Fn(f) = b
                && free::mentions(&f.ret, x)
            {
                return Some(format!("a closure's result, `{}`", f.ret));
            }
            continue;
        }
        if free::mentions(a, x) || free::mentions(b, x) {
            return Some(format!("`{a} = {b}`"));
        }
    }
    None
}

impl Walk<'_> {
    /// What the values of `v` (of type `t`) at each position are, as a
    /// tree: of each of `t`'s positions as deep as `DEPTH`.
    pub(super) fn knowledge(&mut self, v: &Val, t: &Type) -> Tree {
        let mut out = Vec::new();
        for pos in self.positions(t) {
            let cases = self.values_at(v, t, &pos);
            if cases.iter().all(|(g, c)| g.is_empty() && c.is_any()) {
                continue;
            }
            out.push((pos, Disj(cases)));
        }
        out
    }

    /// What may be written at each position of `v`, of type `t`, where
    /// it's invariant: what's declared of the place a reference was made
    /// to, which it says (`trees`); or, of a value that isn't a reference,
    /// what its values there are.
    pub(super) fn written_tree(&mut self, v: &Val, t: &Type) -> Tree {
        if let (Type::Ref(..), Some(o)) = (t, Self::obj_of(v)) {
            return self.trees.get(&o).cloned().unwrap_or_default();
        }
        let mut out = Vec::new();
        for pos in self.positions(t) {
            if let Some((_, Variance::Inv)) = self.type_at(t, &pos) {
                let cases = self.values_at(v, t, &pos);
                out.push((pos, Disj(cases)));
            }
        }
        out
    }

    /// What a call's type parameter `x`, of a function of scheme `s`,
    /// whose type arguments are `targs` there, is: of each position of the
    /// type it is there, each case of each value of it at that position
    /// in the arguments `vals` (lemma A3). An argument that lends a place
    /// `&mut` (`places`) gives what's declared of the place (`written`)
    /// where the parameter's type is invariant, as that's what the callee
    /// may find there, and each such place needs what the instantiation is
    /// to have what's declared there (`Behind`).
    fn instantiate_one(
        &mut self,
        s: &crate::types::Scheme,
        x: &str,
        targs: &crate::types::Subst,
        vals: &[Val],
        written: &[Option<Tree>],
        behind: &mut Vec<Behind>,
    ) -> Inst {
        let Type::Fn(sig) = &s.ty else {
            return Inst::default();
        };
        let ty = self.cx.solved(&targs.apply(&s.params.ty(x)));
        let mut all = vec![Vec::new()];
        all.extend(self.positions(&ty));
        let mut inst = Inst {
            at: all.iter().map(|p| (p.clone(), Vec::new())).collect(),
            any: makes(&self.cx.traits, s, x),
        };
        let push = |inst: &mut Inst, pos: &[usize], c: (Source, Vec<Formula>, Cand)| {
            if let Some((_, cases)) = inst.at.iter_mut().find(|(p, _)| p == pos) {
                cases.push(c);
            }
        };
        for (j, a) in sig.params.iter().enumerate() {
            let Some(v) = vals.get(j).cloned() else {
                continue;
            };
            let (occ, hidden) = self.occurrences(a, x);
            if hidden && inst.any.is_none() {
                inst.any = Some(format!("`{a}`, which `{x}` is in as more than a type argument"));
            }
            if occ.is_empty() {
                continue;
            }
            let at = self.cx.solved(&targs.apply(a));
            for (rho, var) in occ {
                match var {
                    Variance::Co | Variance::Bi => {
                        for pos in &all {
                            let mut p = rho.clone();
                            p.extend_from_slice(pos);
                            for (g, c) in self.values_at(&v, &at, &p) {
                                push(&mut inst, pos, (Source { arg: j, behind: false }, g, c));
                            }
                        }
                    }
                    Variance::Inv => {
                        let decl = match &written[j] {
                            Some(t) => t.clone(),
                            None => self.written_tree(&v, &at),
                        };
                        let mut declared = Vec::new();
                        for pos in &all {
                            let mut p = rho.clone();
                            p.extend_from_slice(pos);
                            let here: Vec<Disj> = (decl.iter())
                                .filter(|(q, _)| *q == p)
                                .map(|(_, d)| d.clone())
                                .collect();
                            let c = match here.is_empty() {
                                true => Cand::Any,
                                false => Cand::All(here.clone()),
                            };
                            push(&mut inst, pos, (Source { arg: j, behind: true }, Vec::new(), c));
                            for d in here {
                                declared.push((pos.clone(), d));
                            }
                        }
                        behind.push(Behind {
                            arg: j,
                            at: rho,
                            declared,
                        });
                    }
                    Variance::Contra => {}
                }
            }
        }
        inst
    }
}

/// What a call says of the positions of what it gives, and of what's behind
/// each `&mut` it's lent, once it's done (`Walk::call_trees`).
#[derive(Debug, Default)]
pub(super) struct CallTrees {
    pub(super) result: Tree,
    pub(super) lent: Vec<(Place, Tree)>,
}

impl Walk<'_> {
    /// Checks what a call of function `symbol` (resolved, whose type
    /// arguments are `targs`, and refinements `sig`) needs of the
    /// refinements of type arguments in its arguments `vals`, the places it
    /// lends `&mut` being `places`, and gives what it says of its result
    /// and of those places after (lemmas A2, A3, A4).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn call_trees(
        &mut self,
        symbol: &str,
        targs: &[Type],
        sig: Option<&RefinedSig>,
        vals: &[Val],
        places: &[Option<Place>],
        spans: &[Span],
        at: Span,
    ) -> Result<CallTrees, Error> {
        let debug = std::env::var("T60").is_ok_and(|w| {
            self.cx.current.is_some_and(|id| self.cx.fns[id.0].symbol.contains(&w))
        });
        if debug {
            eprintln!("call_trees {symbol} {targs:?} vals={vals:?}");
        }
        let known = self.cx.fns.iter().any(|f| f.symbol == symbol)
            || (self.cx.traits.traits.values()).any(|t| t.methods.iter().any(|m| m.symbol == symbol));
        if !known {
            return Ok(CallTrees::default());
        }
        let s = self.cx.scheme(symbol).clone();
        let Type::Fn(fsig) = &s.ty else {
            return Ok(CallTrees::default());
        };
        let names = s.params.as_slice().to_vec();
        if names.len() != targs.len() {
            return Ok(CallTrees::default());
        }
        let subst = crate::types::Subst::new(&names, targs);
        let (fname, pnames) = match self.cx.fns.iter().find(|f| f.symbol == symbol) {
            Some(f) => (f.name.name.clone(), f.params.clone()),
            None => (symbol.rsplit("::").next().unwrap_or(symbol).to_string(), Vec::new()),
        };
        let pname = |j: usize| match pnames.get(j) {
            Some(n) => n.clone(),
            None => format!("{}", j + 1),
        };
        // What's declared of each place lent `&mut`, as of the argument.
        let mut written: Vec<Option<Tree>> = Vec::new();
        for p in places.iter().take(vals.len()) {
            written.push(p.as_ref().map(|p| under(&self.place_decl(p), 0)));
        }
        written.resize(vals.len(), None);
        // The parameters' types there, and what's written of them.
        let ptys: Vec<Type> = (fsig.params.iter())
            .map(|t| self.cx.solved(&subst.apply(t)))
            .collect();
        let env_targs: Vec<(String, Type)> = names.iter().cloned().zip(targs.to_vec()).collect();
        let mut param_trees: Vec<Tree> = Vec::new();
        for j in 0..fsig.params.len() {
            let r = sig.and_then(|s| s.params.get(j).cloned().flatten());
            let tree = match r {
                Some(r) if r.has_carried() => self.carried_tree(&r, Env::Call(vals, &env_targs)),
                _ => Vec::new(),
            };
            param_trees.push(tree);
        }
        // What each argument must have, of what's written in the
        // parameter's type: each value there has it.
        for (j, tree) in param_trees.iter().enumerate() {
            let (Some(v), Some(t)) = (vals.get(j), ptys.get(j)) else {
                continue;
            };
            let what = format!("argument `{}` of `{fname}`", pname(j));
            let span = spans.get(j).copied().unwrap_or(at);
            self.need_tree(v, t, tree, None, span, &what)?;
        }
        // Each type parameter's instantiation, and what it needs of what's
        // behind each `&mut`.
        let mut insts: Vec<(String, Inst, Vec<Behind>)> = Vec::new();
        for x in &names {
            if !s.params.is_type(x) {
                continue;
            }
            let mut behind = Vec::new();
            let inst = self.instantiate_one(&s, x, &subst, vals, &written, &mut behind);
            insts.push((x.clone(), inst, behind));
        }
        if debug {
            for (x, inst, _) in &insts {
                eprintln!("  inst {x} = {:?}", inst);
            }
            eprintln!("  written {:?}", written);
        }
        for (x, inst, behind) in &insts {
            let xt = self.cx.solved(&subst.apply(&s.params.ty(x)));
            for b in behind {
                for (pos, want) in &b.declared {
                    if want.is_any() {
                        continue;
                    }
                    let Some((pt, _)) = self.type_at(&xt, pos) else {
                        continue;
                    };
                    let span = spans.get(b.arg).copied().unwrap_or(at);
                    let text = want.text().unwrap_or_default();
                    let behind_msg = format!(
                        "can't prove `{text}` of what `{fname}` may write behind argument `{}`",
                        pname(b.arg)
                    );
                    if let Some(why) = &inst.any {
                        return Err(Error::ty(
                            span,
                            format!("{behind_msg}: {why} can make or change a value of it"),
                        ));
                    }
                    let cases = (inst.at.iter())
                        .find(|(p, _)| p == pos)
                        .map(|(_, c)| c.clone())
                        .unwrap_or_default();
                    for (src, g, c) in cases {
                        let one = [(g, c)];
                        if self.implies(&one, want, &pt).is_err() {
                            let span = spans.get(src.arg).copied().unwrap_or(at);
                            let msg = match src.behind {
                                true => behind_msg.clone(),
                                false => format!(
                                    "can't prove `{text}` for argument `{}` of `{fname}`",
                                    pname(src.arg)
                                ),
                            };
                            let _ = x;
                            return Err(Error::ty(span, msg));
                        }
                    }
                }
            }
        }
        // Where a parameter's type is invariant but not as a type
        // parameter's, what's written there must be exactly what's declared
        // of the argument there: what the callee may write is what its own
        // type says.
        for (j, a) in fsig.params.iter().enumerate() {
            let (Some(v), Some(t)) = (vals.get(j).cloned(), ptys.get(j).cloned()) else {
                continue;
            };
            let mut inside: Vec<Pos> = Vec::new();
            for (x, ..) in &insts {
                let (occ, _) = self.occurrences(a, x);
                inside.extend(occ.into_iter().map(|(p, _)| p));
            }
            let decl = match &written[j] {
                Some(d) => d.clone(),
                None => self.written_tree(&v, &t),
            };
            for (pos, want) in &decl {
                if want.is_any() || inside.iter().any(|p| pos.starts_with(p)) {
                    continue;
                }
                let Some((pt, Variance::Inv)) = self.type_at(&t, pos) else {
                    continue;
                };
                let have: Vec<(Vec<Formula>, Cand)> = (param_trees[j].iter())
                    .filter(|(p, _)| p == pos)
                    .map(|(_, d)| (Vec::new(), Cand::All(vec![d.clone()])))
                    .collect();
                let have = match have.is_empty() {
                    true => vec![(Vec::new(), Cand::Any)],
                    false => have,
                };
                if self.implies(&have, want, &pt).is_err() {
                    let text = want.text().unwrap_or_default();
                    let span = spans.get(j).copied().unwrap_or(at);
                    return Err(Error::ty(
                        span,
                        format!(
                            "can't prove `{text}` of what `{fname}` may write behind argument `{}`",
                            pname(j)
                        ),
                    ));
                }
            }
        }
        // What it gives, and leaves behind what it's lent.
        let mut out = CallTrees::default();
        let ret = (*fsig.ret).clone();
        for (x, inst, _) in &insts {
            let (occ, _) = self.occurrences(&ret, x);
            for (rho, _) in occ {
                for (pos, d) in inst.tree() {
                    let mut p = rho.clone();
                    p.extend(pos);
                    out.result.push((p, d));
                }
                if inst.any.is_none() && inst.at.iter().all(|(p, _)| !p.is_empty()) {
                    out.result.push((rho.clone(), inst.disj(&[])));
                }
            }
        }
        for (j, p) in places.iter().enumerate() {
            let Some(p) = p else {
                continue;
            };
            let Some(Type::Ref(_, true, u)) = fsig.params.get(j) else {
                continue;
            };
            let mut tree = below(&param_trees[j], 0);
            for (x, inst, _) in &insts {
                let (occ, _) = self.occurrences(u, x);
                for (rho, _) in occ {
                    for (pos, d) in inst.tree() {
                        let mut q = rho.clone();
                        q.extend(pos);
                        tree.push((q, d));
                    }
                }
            }
            if !tree.is_empty() {
                out.lent.push((p.clone(), tree));
            }
        }
        Ok(out)
    }
}

impl Walk<'_> {
    /// Says of array `v`, of type `t`, that each of its elements is one of
    /// `items`, at each of their positions.
    pub(super) fn elements(&mut self, v: &Val, t: &Type, items: &[Val]) {
        let Type::Array(elem, _) = t else {
            return;
        };
        let elem = self.cx.solved(elem);
        let mut all = vec![Vec::new()];
        all.extend(self.positions(&elem));
        let mut tree = Vec::new();
        for pos in all {
            let mut cases = Vec::new();
            for item in items {
                cases.extend(self.values_at(item, &elem, &pos));
            }
            if cases.iter().any(|(g, c)| g.is_empty() && c.is_any()) {
                continue;
            }
            tree.push((under(&vec![(pos, Disj(cases))], 0)).remove(0));
        }
        self.assume_tree(v, t, &tree);
    }
}
