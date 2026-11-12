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
use crate::traits::{COPY, DISCARD, SEND, SYNC};
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
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Disj(pub(super) Vec<(Vec<Formula>, Cand)>);

impl PartialEq for Cand {
    /// The same, as far as is cheap to tell: predicates by identity.
    fn eq(&self, other: &Cand) -> bool {
        match (self, other) {
            (Cand::Exact(a), Cand::Exact(b)) => a == b,
            (Cand::Pred(a), Cand::Pred(b)) => Rc::ptr_eq(a, b),
            (Cand::All(a), Cand::All(b)) => a == b,
            (Cand::Any, Cand::Any) => true,
            _ => false,
        }
    }
}

impl Disj {
    fn any() -> Disj {
        Disj(vec![(Vec::new(), Cand::Any)])
    }

    fn one(c: Cand) -> Disj {
        Disj(vec![(Vec::new(), c)])
    }

    /// Does it say nothing, so that any value has it?
    pub(super) fn is_any(&self) -> bool {
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
    pub(super) fn text(&self) -> Option<String> {
        self.0.iter().find_map(|(_, c)| c.text())
    }

    /// How many cases and conditions it has, at every depth.
    fn size(&self) -> u64 {
        let cand = |c: &Cand| match c {
            Cand::All(ds) => ds.iter().map(Disj::size).sum(),
            _ => 1,
        };
        (self.0.iter()).map(|(g, c)| g.len() as u64 + cand(c)).sum()
    }
}

impl Cand {
    /// What all of `ds` say, flattened: an `All` in an `All` is spliced
    /// in, what says nothing is left out, and each is said once.
    pub(super) fn all(ds: Vec<Disj>) -> Cand {
        let mut out: Vec<Disj> = Vec::new();
        let push = |out: &mut Vec<Disj>, d: Disj| {
            if !d.is_any() && !out.contains(&d) {
                out.push(d);
            }
        };
        for d in ds {
            match <[_; 1]>::try_from(d.0) {
                Ok([(g, Cand::All(es))]) if g.is_empty() => {
                    for e in es {
                        push(&mut out, e);
                    }
                }
                Ok([one]) => push(&mut out, Disj(vec![one])),
                Err(cases) => push(&mut out, Disj(cases)),
            }
        }
        match <[_; 1]>::try_from(out) {
            Ok([d]) => match <[_; 1]>::try_from(d.0) {
                Ok([(g, c)]) if g.is_empty() => c,
                Ok([one]) => Cand::All(vec![Disj(vec![one])]),
                Err(cases) => Cand::All(vec![Disj(cases)]),
            },
            Err(out) if out.is_empty() => Cand::Any,
            Err(out) => Cand::All(out),
        }
    }

    /// Is it of no value, as what's in `None`'s `Some` or an empty list's
    /// elements: one of a `Disj` with no cases?
    fn is_none(&self) -> bool {
        match self {
            Cand::All(ds) => ds.iter().any(|d| d.0.iter().all(|(_, c)| c.is_none())),
            _ => false,
        }
    }

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

/// One case of an instantiation's at a position: the argument it's of, its
/// conditions, and what the value is.
type Case = (Source, Vec<Formula>, Cand);

/// What a call's type parameter `X` is (`Walk::instantiate`): at each
/// position of the type it's given, the cases, each from an argument, and
/// why it may be anything, if it may.
#[derive(Debug, Clone, Default)]
pub(super) struct Inst {
    at: Vec<(Pos, Vec<Case>)>,
    /// Why nothing's known of it: a bound that can make one.
    any: Option<String>,
    /// Why nothing's known of it at the positions where the type it's
    /// given is invariant (behind a `&mut`, in an `Unsafe`): a bound whose
    /// methods take it and give none back, but can write there (lemma A5
    /// of docs/68-fail-closed.md); and those positions.
    changed: Option<(String, Vec<Pos>)>,
}

impl Inst {
    /// Why nothing's known of it at position `pos`, if nothing is.
    fn unknown_at(&self, pos: &[usize]) -> Option<&String> {
        if let Some(why) = &self.any {
            return Some(why);
        }
        match &self.changed {
            Some((why, inv)) if inv.iter().any(|p| pos.starts_with(p)) => Some(why),
            _ => None,
        }
    }

    /// What it says of position `pos`.
    fn disj(&self, pos: &[usize]) -> Disj {
        if self.unknown_at(pos).is_some() {
            return Disj::any();
        }
        match self.at.iter().find(|(p, _)| p == pos) {
            Some((_, cases)) => Disj(
                cases
                    .iter()
                    .map(|(_, g, c)| (g.clone(), c.clone()))
                    .collect(),
            ),
            None => Disj::default(),
        }
    }

    /// It as a tree, of the type it's given.
    fn tree(&self) -> Tree {
        if self.any.is_some() {
            return Vec::new();
        }
        self.at
            .iter()
            .filter(|(p, _)| self.unknown_at(p).is_none())
            .map(|(p, _)| (p.clone(), self.disj(p)))
            .collect()
    }
}

/// A check a call needs of what's behind a `&mut` (or in an `Unsafe`) an
/// argument gives it: that what the callee may write there, `inst` at each
/// position below `at` in the parameter, has what the place must keep.
struct Behind {
    arg: usize,
    /// What's declared at each position of what the parameter's type
    /// parameter is there.
    declared: Tree,
}

/// Is trait `name` one whose values only ever go in, never out: a marker,
/// or a trait each of whose methods takes `Self` only as `self`, `&self` or
/// a `&Self`, gives back nothing with `Self` in it, has no associated type,
/// and whose supertraits are such too? Its methods can make no value of
/// `Self`, nor change one (lemma A5). Not `Int`, whose literals make values,
/// nor `Atom`, which `Volatile::at` reads from an address.
pub(super) fn consumer(traits: &crate::traits::Traits, name: &str, seen: &mut Vec<String>) -> bool {
    use crate::traits::{FN_TRAITS, INT, SIGNED, UNSIGNED};
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
        info.variances(symbol)
            .map(<[Variance]>::to_vec)
            .unwrap_or_default()
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

    /// `type_at`, of a position something is said or declared at, of a
    /// value of type `t`, at `span`: one the walk can't follow there
    /// would go unchecked, so it refuses (lemma F2 of
    /// docs/68-fail-closed.md). Skipping it is never right: a missing
    /// case refuses rather than says nothing's declared.
    pub(super) fn followed(
        &mut self,
        t: &Type,
        pos: &[usize],
        span: Span,
    ) -> Option<(Type, Variance)> {
        let found = self.type_at(t, pos);
        if found.is_none() {
            self.failed(Error::ty(
                span,
                format!("can't tell what's declared inside a `{t}` here: the checker can't follow its type"),
            ));
        }
        found
    }

    /// Every position of `t` where it's invariant: behind a `&mut`, in an
    /// `Unsafe`.
    pub(super) fn inv_positions(&mut self, t: &Type) -> Vec<Pos> {
        let mut out = Vec::new();
        for p in self.positions(t) {
            if let Some((_, Variance::Inv)) = self.type_at(t, &p) {
                out.push(p);
            }
        }
        out
    }

    /// Every position of `t` but the whole value's. A type is a finite
    /// tree, so there are as many as it has nodes: none is left out, as an
    /// obligation at one left out wouldn't be checked.
    fn positions(&mut self, t: &Type) -> Vec<Pos> {
        fn go(w: &mut Walk, t: &Type, at: &mut Pos, out: &mut Vec<Pos>) {
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
        go(
            self,
            t,
            x,
            &mut Vec::new(),
            Variance::Co,
            &mut out,
            &mut hidden,
        );
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
                if symbol == crate::check::UNSAFE
                    || crate::intrinsic::is_intrinsic_type(symbol) =>
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
                    crate::types::AdtKind::Enum(vs) => {
                        vs.iter().map(|(_, ts)| ts.clone()).collect()
                    }
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
        obj_of(v)
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
    pub(super) fn values_at(
        &mut self,
        v: &Val,
        t: &Type,
        pos: &[usize],
    ) -> Vec<(Vec<Formula>, Cand)> {
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
                    (g, Cand::all(all))
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
                if k == 0 && !self.no_place_reads =>
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
                false => vec![(Vec::new(), Cand::all(reg))],
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
            if pos.is_empty()
                && let Some(f) = self.disj_formula(d, v, false)
            {
                self.fact(f);
            }
            if let Some(o) = Self::obj_of(v) {
                let tree = self.trees.entry(o).or_default();
                if !tree.iter().any(|(p, e)| p == pos && e == d) {
                    logic::count(|w| w.carried += d.size());
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
                    self.assume_tree_depth(
                        &w,
                        &inner,
                        &vec![(rest.to_vec(), d.clone())],
                        depth + 1,
                    );
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
            self.failed(Error::ty(
                span,
                format!("can't prove `{}` for {what}", part.text()),
            ));
            return Ok(());
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
            let Some((pt, var)) = self.followed(t, pos, span) else {
                continue;
            };
            let cases = self.values_at(v, t, pos);
            if let Err(reason) = self.implies(&cases, d, &pt) {
                let text = d.text().unwrap_or_default();
                let failure = self.proof_failure(reason, None, None, &text);
                self.failed(
                    failure.attach(Error::ty(span, format!("can't prove `{text}` for {what}"))),
                );
                // Assumed from here on, as if it had been proved.
                self.assume_tree(v, t, tree);
                return Ok(());
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
                        self.failed(Error::ty(
                            span,
                            format!("can't prove `{text}` of what may be written through {what}"),
                        ));
                        return Ok(());
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
            let Some((pt, Variance::Inv)) = self.followed(t, pos, span) else {
                continue;
            };
            if self.implies(&[(Vec::new(), Cand::Any)], want, &pt).is_err() {
                let text = want.text().unwrap_or_default();
                self.failed(Error::ty(
                    span,
                    format!("can't prove `{text}` of what may be written through {what}"),
                ));
                return Ok(());
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
        self.place_decl_depth(p, true)
    }

    /// What's said of a value read at place `p` that's new: what's
    /// declared there, but for its own struct field refinements, which
    /// `assume_fields` says of all the fields' values (as reading them here
    /// would read this one again); and what the call that last lent it left
    /// there (`lent_trees`).
    /// What's declared of the value at place `p`, but for the field
    /// refinements of the structs it's in, which `place_decl` reads the
    /// structs' fields for: so reading nothing.
    pub(super) fn place_decl_own(&mut self, p: &Place) -> Tree {
        self.place_decl_depth(p, false)
    }

    pub(super) fn place_read(&mut self, p: &Place) -> Tree {
        let mut tree = self.place_decl_depth(p, false);
        // What's declared of the value itself there isn't known of it until
        // it's proved again (`Walk::post`).
        if self.reproving.contains(p) {
            tree.retain(|(pos, _)| !pos.is_empty());
        }
        // Nor, of what it's in, what's declared at the positions it's at:
        // reading the tuple a lent element is in would say it again
        // (lemma W3 of docs/63-what-is-declared.md).
        let pt = self.place_ty(p);
        let mut toward: Vec<usize> = Vec::new();
        for r in self.reproving.clone() {
            if r.root != p.root
                || r.path.len() <= p.path.len()
                || r.path[..p.path.len()] != p.path[..]
            {
                continue;
            }
            match r.path[p.path.len()] {
                Proj::Deref => toward.push(0),
                Proj::Field(j) => {
                    for k in 0..Self::arity(&pt) {
                        if self.fields_with(&pt, k).iter().any(|(_, f, ..)| *f == j) {
                            toward.push(k);
                        }
                    }
                }
            }
        }
        if !toward.is_empty() {
            tree.retain(|(pos, _)| pos.first().is_none_or(|k| !toward.contains(k)));
        }
        if let Some(t) = self.lent_trees.get(p) {
            tree.extend(t.iter().cloned());
        }
        tree
    }

    fn place_decl_depth(&mut self, p: &Place, fields: bool) -> Tree {
        let Some((last, init)) = p.path.split_last() else {
            return self.decl.get(&p.root).cloned().unwrap_or_default();
        };
        let q = Place {
            root: p.root,
            path: init.to_vec(),
        };
        let qt = self.place_ty(&q);
        let outer = self.place_decl_depth(&q, fields);
        match last {
            Proj::Deref => {
                let mut out = below(&outer, 0);
                // What the reference itself says; or what's said of the box:
                // a write through a box gives it a new value, so it's what's
                // in it now (`Walk::kill`).
                if let Type::Ref(..) | Type::Box(_) = qt
                    && let Some(&o) = self.objs.get(&q)
                {
                    let reg = self.trees.get(&o).cloned().unwrap_or_default();
                    out.extend(below(&reg, 0));
                }
                out
            }
            Proj::Field(j) => {
                let vals = match fields {
                    true => Some(self.fields_of(&q, &qt).unwrap_or_default()),
                    false => None,
                };
                self.field_decl(&outer, &qt, *j, vals.as_deref())
            }
        }
    }

    /// What's declared of field `j` of a struct of type `qt`, of which
    /// `outer` is what's declared: at the field's positions, what's
    /// declared of the struct's type arguments there; and, given the
    /// struct's fields' values `vals`, the carried parts of the field's own
    /// refinements, of them.
    pub(super) fn field_decl(
        &mut self,
        outer: &Tree,
        qt: &Type,
        j: usize,
        vals: Option<&[Val]>,
    ) -> Tree {
        let mut out = self.variant_decl(outer, qt, 0, j);
        // The field's own refinements: all of them, if they're about it
        // alone, as a variant's field's are (D6 of
        // docs/67-refinement-aliases.md); or else the carried parts, of
        // what it's in.
        if let Some(fields) = self.refined_fields(qt).cloned()
            && let Some(Some(r)) = fields.refs.get(j)
        {
            let args = self.adt_args(qt);
            if fields.alone(j) {
                out.extend(self.own_tree(r, Env::Fields(&[], &args)));
            } else if let Some(vals) = vals
                && r.has_carried()
            {
                out.extend(self.carried_tree(r, Env::Fields(vals, &args)));
            }
        }
        out
    }

    /// What refinement `r`, of a field about it alone, says of each
    /// position of the field's value, with names as `env` says: each of its
    /// parts at its position, the whole value's at `[]`.
    fn own_tree(&mut self, r: &Refinement, env: Env) -> Tree {
        let mut out = Vec::new();
        for part in &r.parts {
            if let Some(c) = self.close(&part.pred, env) {
                out.push((part.path.clone(), Disj::one(Cand::Pred(c))));
            }
        }
        out
    }

    /// What's declared of field `j` of variant `v` (of a struct, `0`) of a
    /// value of type `qt`, of which `outer` is what's declared: at the
    /// field's positions, what's declared of the type's arguments there.
    /// Not what's declared of the whole value (position `[]`): that's of
    /// the value the field is in, which a write to it changes
    /// (`Walk::keep_wholes`), and of an enum's, only its variant is
    /// anything to a refinement (by its measures), which no write to its
    /// field changes.
    pub(super) fn variant_decl(&mut self, outer: &Tree, qt: &Type, v: usize, j: usize) -> Tree {
        let mut out = Vec::new();
        for (pos, d) in outer {
            let Some((&k, rest)) = pos.split_first() else {
                continue;
            };
            for (w, f, rho, _) in self.fields_with(qt, k) {
                if w == v && f == j {
                    let mut at = rho;
                    at.extend_from_slice(rest);
                    out.push((at, d.clone()));
                }
            }
        }
        // An enum's variant's field's own refinement, if it's about it
        // alone: what every value there has, which nothing changes.
        if let Some(rv) = self.refined_variants(qt).cloned()
            && let Some(variant) = rv.variants.get(v)
            && let Some(Some(r)) = variant.refs.get(j)
            && variant.alone(j)
        {
            let args = self.adt_args(qt);
            out.extend(self.own_tree(r, Env::Fields(&[], &args)));
        }
        out
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
            Cand::All(ds) => Cand::all(ds.iter().map(disj).collect()),
        }
    }
    fn disj(d: &Disj) -> Disj {
        // A case with no values in it adds none: it's left out, as it would
        // be of the disjunction it's in (`Cand::is_none`), while another is
        // there. With none other, what each says is kept.
        let some = d.0.iter().any(|(_, c)| !c.is_none());
        let cases: Vec<(Vec<Formula>, Cand)> = (d.0.iter())
            .filter(|(_, c)| !some || !c.is_none())
            .map(|(g, c)| (g.clone(), cand(c)))
            .collect();
        // Nothing said in any case, or no values: nothing declared.
        if cases.iter().all(|(_, c)| c.is_any()) {
            return Disj::any();
        }
        Disj(cases)
    }
    (tree.iter())
        .map(|(p, d)| (p.clone(), disj(d)))
        .filter(|(_, d)| !d.is_any())
        .collect()
}

/// Is the impl of trait `tr` for `t` (as `select` finds it) generic in
/// every type argument of `t`, each a type parameter of its own, which only
/// markers and consumers bound? Then its methods can't make or change a
/// value at a position of `t` but as code generic in them can (lemma A2):
/// so `T: Iterator`, of a `T` that's an `Iter<'_, U>`, keeps what's said of
/// its `U`s, and what `<T as Iterator>::Item` is says what it gives
/// (`Walk::subst_tree`).
fn parametric_impl(cx: &Checker, t: &Type, tr: &str) -> bool {
    parametric_impl_seen(cx, t, tr, &mut Vec::new())
}

/// `parametric_impl`, inside `drop_changes`, which has seen the types
/// `seen` on the way.
fn parametric_impl_seen(cx: &Checker, t: &Type, tr: &str, seen: &mut Vec<Type>) -> bool {
    parametric_impl_depth(cx, t, tr, 0, seen)
}

/// `parametric_impl`, as deep as `depth`: a bound of the impl's on one of
/// its parameters that isn't a consumer is as good, if the impl of it for
/// what the parameter is there is parametric too (`Rev<I>`'s `I:
/// DoubleEndedIterator`, of an `Iter<'_, T>`): its methods can make no more
/// than code generic in the parameter can, so neither can these.
fn parametric_impl_depth(
    cx: &Checker,
    t: &Type,
    tr: &str,
    depth: usize,
    dropping: &mut Vec<Type>,
) -> bool {
    let traits = &cx.traits;
    let Some((i, impl_args)) = traits.select(t, Some(tr), &[]) else {
        return false;
    };
    let def = &traits.impls[i];
    let Type::Adt(_, _, args) = &def.self_ty else {
        return false;
    };
    let mut seen: Vec<&str> = Vec::new();
    for a in args {
        let Type::Param(p, _) = a else {
            return false;
        };
        if seen.contains(&p.as_str()) {
            return false;
        }
        seen.push(p);
    }
    let names = def.params.as_slice();
    def.preds.iter().all(|p| {
        // A bound on a type with a parameter in it, `I::Item: Int`, can
        // make values of it unless it's a consumer, as `makes` says.
        let Type::Param(q, _) = &p.ty else {
            return !seen.iter().any(|q| free::mentions(&p.ty, q))
                || consumer(traits, &p.trait_name, &mut Vec::new())
                || crate::traits::FN_TRAITS.contains(&p.trait_name.as_str());
        };
        let arg = names
            .iter()
            .position(|n| n == q)
            .and_then(|k| impl_args.get(k));
        if !seen.contains(&q.as_str()) {
            return true;
        }
        // A consumer can't make one, but can change what's inside one
        // where it's invariant (`changes`); so can dropping one, by a
        // `discard` that isn't parametric (`drop_changes`).
        if consumer(traits, &p.trait_name, &mut Vec::new()) {
            let Some(a) = arg else {
                return false;
            };
            return match p.trait_name.as_str() {
                COPY | SEND | SYNC => true,
                DISCARD => !writable_inside(cx, a) || drop_changes_seen(cx, a, dropping).is_none(),
                _ => !writable_inside(cx, a),
            };
        }
        depth < DEPTH
            && arg.is_some_and(|a| parametric_impl_depth(cx, a, &p.trait_name, depth + 1, dropping))
    }) && def.eqs.iter().all(|(a, b)| {
        let mentions = |t: &Type| seen.iter().any(|q| free::mentions(t, q));
        matches!(a, Type::Proj(base, ..) if !matches!(**base, Type::Param(..)))
            || !mentions(a) && !mentions(b)
    })
}

/// Why a type parameter's values may be changed where the type it's given
/// is invariant inside, which a call can't follow: a bound on it, or on a
/// type it's in, by a consumer that isn't a marker. A consumer's method
/// gives back no value of it, but an impl of it for a `&mut` can write
/// through it, and one for a `Cell` can set it (lemma A5 of
/// docs/68-fail-closed.md). `Discard` is such a consumer where dropping
/// what it is runs a `discard` that can (`drop_changes`, lemma A9): `inst`
/// says what a bound's type is at the call, if it's known. `None` if
/// there's none.
pub(super) fn changes(
    cx: &Checker,
    s: &crate::types::Scheme,
    x: &str,
    inst: &dyn Fn(&Type) -> Option<Type>,
) -> Option<String> {
    let traits = &cx.traits;
    for p in &s.preds {
        if !free::mentions(&p.ty, x) || [COPY, SEND, SYNC].contains(&p.trait_name.as_str()) {
            continue;
        }
        if p.trait_name == DISCARD {
            match inst(&p.ty) {
                Some(t) => match drop_changes(cx, &t) {
                    Some(why) => return Some(why),
                    None => continue,
                },
                None => return Some(format!("dropping a `{}`", p.ty)),
            }
        }
        if consumer(traits, &p.trait_name, &mut Vec::new()) {
            let name = traits.shorten(&p.trait_name);
            return Some(format!("`{}: {name}`", p.ty));
        }
    }
    None
}

/// Why dropping a value of type `t` can change a value at one of its
/// positions where it's invariant (behind a `&mut`, in an `Unsafe`), if it
/// can: it runs a `discard` that's written, not derived, and isn't
/// parametric in what's there (`parametric_impl`), which may write what it
/// likes there (lemma A9 of docs/68-fail-closed.md). A derived `Discard`
/// drops the parts; a type parameter's, an opaque type's, are checked where
/// what they are is picked (`changes`).
pub(super) fn drop_changes(cx: &Checker, t: &Type) -> Option<String> {
    drop_changes_seen(cx, t, &mut Vec::new())
}

/// `drop_changes`, having seen the types `seen` on the way: a type that
/// holds itself, by a box, changes nothing its parts don't.
fn drop_changes_seen(cx: &Checker, t: &Type, seen: &mut Vec<Type>) -> Option<String> {
    if seen.contains(t) {
        return None;
    }
    if seen.len() > DEPTH {
        return Some(format!("dropping a `{t}`"));
    }
    seen.push(t.clone());
    let out = match t {
        Type::Adt(symbol, _, args) if cx.traits.own_discard(symbol) => {
            match parametric_impl_seen(cx, t, DISCARD, seen) {
                true => None,
                // The `discard` it comes to, if it's one of what's in it:
                // `List<W>`'s drops each `W`.
                false => (args.iter())
                    .find_map(|a| drop_changes_seen(cx, &cx.solved(a), seen))
                    .or_else(|| {
                        Some(format!(
                            "`{}`'s `discard`",
                            cx.traits.shorten(&t.to_string())
                        ))
                    }),
            }
        }
        Type::Adt(symbol, ..) => {
            let parts: Vec<Type> = match cx.adts.get(symbol).map(|d| &d.kind) {
                Some(crate::types::AdtKind::Struct(_)) => (struct_fields(&cx.adts, t))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(_, f)| f)
                    .collect(),
                Some(crate::types::AdtKind::Enum(vs)) => (0..vs.len())
                    .flat_map(|v| crate::types::variant_fields(&cx.adts, t, v))
                    .collect(),
                None => Vec::new(),
            };
            parts
                .iter()
                .find_map(|f| drop_changes_seen(cx, &cx.solved(f), seen))
        }
        Type::Box(inner) | Type::Array(inner, _) => drop_changes_seen(cx, &cx.solved(inner), seen),
        Type::Tuple(ts) => ts
            .iter()
            .find_map(|u| drop_changes_seen(cx, &cx.solved(u), seen)),
        _ => None,
    };
    seen.pop();
    out
}

/// Can something be written at a position inside a value of type `t`: is
/// `t` invariant in one of its positions, as behind a `&mut` or in an
/// `Unsafe`? A type parameter, an opaque type or a projection may be
/// anything, so yes.
pub(super) fn writable_inside(cx: &Checker, t: &Type) -> bool {
    fn go(cx: &Checker, t: &Type, depth: usize) -> bool {
        if depth > DEPTH {
            return true;
        }
        match t {
            Type::Ref(_, true, _) => true,
            Type::Ref(_, false, inner)
            | Type::Box(inner)
            | Type::Slice(inner)
            | Type::Array(inner, _) => go(cx, &cx.solved(inner), depth + 1),
            Type::Tuple(ts) => ts.iter().any(|u| go(cx, &cx.solved(u), depth + 1)),
            Type::Adt(symbol, lts, args) => {
                let info = cx.adt_info();
                let vs = info.variances(symbol).map(<[Variance]>::to_vec);
                args.iter().enumerate().any(|(k, a)| {
                    match vs.as_ref().and_then(|vs| vs.get(lts.len() + k)) {
                        Some(Variance::Co) => go(cx, &cx.solved(a), depth + 1),
                        Some(Variance::Contra | Variance::Bi) => false,
                        Some(Variance::Inv) | None => true,
                    }
                })
            }
            Type::Param(..) | Type::Opaque(..) | Type::Proj(..) | Type::Var(_) => true,
            _ => false,
        }
    }
    go(cx, t, 0)
}

/// Where a type parameter's values come from that a call can't follow: a
/// bound on it that makes values (`consumer`), or another that might,
/// or a closure's result. `None` if there's none (lemma A2).
pub(super) fn makes(
    cx: &Checker,
    s: &crate::types::Scheme,
    x: &str,
    ty: Option<&Type>,
) -> Option<String> {
    let traits = &cx.traits;
    let this = s.params.ty(x);
    for p in &s.preds {
        if p.ty == this {
            if !consumer(traits, &p.trait_name, &mut Vec::new())
                && !ty.is_some_and(|t| parametric_impl(cx, t, &p.trait_name))
            {
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
            // With no cases, there are no values there: that's known too.
            let cases = Disj(self.values_at(v, t, &pos));
            if cases.is_any() {
                continue;
            }
            out.push((pos, cases));
        }
        out
    }

    /// What may be written at each position of `v`, of type `t`, where
    /// it's invariant: what's declared of the place a reference was made
    /// to, which it says (`trees`); or, of a value that isn't a reference,
    /// what its values there are.
    pub(super) fn written_tree(&mut self, v: &Val, t: &Type) -> Tree {
        if let (Type::Ref(_, _, inner), Some(o)) = (t, Self::obj_of(v)) {
            let mut tree = self.trees.get(&o).cloned().unwrap_or_default();
            // A reference to a value that isn't a place's, as a method's
            // receiver made of a field of what a call gave: what may be
            // written there is at most what's known of it, which came from
            // what said what may be written where it is.
            if let Val::Ref(_, Pointee::Val(w)) = v {
                let (w, inner) = ((**w).clone(), self.cx.solved(inner));
                let known = self.knowledge(&w, &inner);
                tree.extend(under(&widen(&known), 0));
            }
            // And whatever it says, what's declared of the place it's to,
            // if that's known: what's written through it must keep that
            // too (lemma F1 of docs/68-fail-closed.md).
            if let Val::Ref(_, Pointee::Place(p)) = v {
                let p = p.clone();
                tree.extend(under(&self.place_decl(&p), 0));
            }
            return tree;
        }
        // What's declared of what's in it there is what refinements said,
        // not the values there now (`widen`).
        let mut out = Vec::new();
        for pos in self.positions(t) {
            if let Some((_, Variance::Inv)) = self.type_at(t, &pos) {
                let cases = self.values_at(v, t, &pos);
                out.push((pos, Disj(cases)));
            }
        }
        widen(&out)
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
        let cx = self.cx;
        let at_call = |t: &Type| Some(cx.solved(&targs.apply(t)));
        let changed = changes(cx, s, x, &at_call).map(|why| (why, self.inv_positions(&ty)));
        let mut inst = Inst {
            at: all.iter().map(|p| (p.clone(), Vec::new())).collect(),
            any: makes(self.cx, s, x, Some(&ty)),
            changed,
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
                inst.any = Some(format!(
                    "`{a}`, which `{x}` is in as more than a type argument"
                ));
            }
            if occ.is_empty() {
                continue;
            }
            let at = self.cx.solved(&targs.apply(a));
            for (rho, var) in occ {
                match var {
                    Variance::Co | Variance::Bi => {
                        // Where `x`'s own type is invariant, behind a `&mut`
                        // in it, what's there is typed by what's declared
                        // of it, not what's known of the values there now
                        // (lemma A4); and, as for an invariant occurrence,
                        // what the callee may write there must have it: an
                        // `IterMut<'_, T>` given whole lets the callee write
                        // `T`s through what it gives (lemma W5 of
                        // docs/63-what-is-declared.md).
                        let mut declared: Tree = Vec::new();
                        let mut written_here: Option<Tree> = None;
                        for pos in &all {
                            let mut p = rho.clone();
                            p.extend_from_slice(pos);
                            if let Some((_, Variance::Inv)) = self.type_at(&ty, pos) {
                                let w = match &written_here {
                                    Some(w) => w.clone(),
                                    None => {
                                        let w = self.arg_written(j, &v, &at);
                                        let w = self.arg_tree(j, w);
                                        written_here = Some(w.clone());
                                        w
                                    }
                                };
                                let here: Vec<Disj> = (w.iter())
                                    .filter(|(q, _)| *q == p)
                                    .map(|(_, d)| d.clone())
                                    .collect();
                                for d in &here {
                                    declared.push((pos.clone(), d.clone()));
                                }
                                let c = match here.is_empty() {
                                    true => Cand::Any,
                                    false => Cand::all(here),
                                };
                                let src = Source {
                                    arg: j,
                                    behind: true,
                                };
                                push(&mut inst, pos, (src, Vec::new(), c));
                                continue;
                            }
                            for (g, c) in self.arg_values_at(j, &v, &at, &p) {
                                let src = Source {
                                    arg: j,
                                    behind: false,
                                };
                                push(&mut inst, pos, (src, g, c));
                            }
                        }
                        // Of a value, not a place, what's said behind it is
                        // what was known of what it points to when it was
                        // made (an empty slice's elements, none): what's
                        // declared there is what a `let` of it would say.
                        let declared = match written[j] {
                            Some(_) => declared,
                            None => widen(&declared),
                        };
                        if !declared.is_empty() {
                            behind.push(Behind { arg: j, declared });
                        }
                    }
                    Variance::Inv => {
                        let decl = match &written[j] {
                            Some(t) => t.clone(),
                            None => {
                                let w = self.arg_written(j, &v, &at);
                                self.arg_tree(j, w)
                            }
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
                                false => Cand::all(here.clone()),
                            };
                            push(
                                &mut inst,
                                pos,
                                (
                                    Source {
                                        arg: j,
                                        behind: true,
                                    },
                                    Vec::new(),
                                    c,
                                ),
                            );
                            for d in here {
                                declared.push((pos.clone(), d));
                            }
                        }
                        behind.push(Behind { arg: j, declared });
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
        coerced: &[Option<(usize, Type)>],
        at: Span,
    ) -> Result<CallTrees, Error> {
        self.coerced_args = coerced.to_vec();
        let out = self.call_trees_now(symbol, targs, sig, vals, places, spans, at);
        self.coerced_args.clear();
        out
    }

    /// What's known of the values at position `pos` of argument `j`,
    /// `v`, of parameter type `t`: through the references and boxes a
    /// coercion looked through (`&Box<T>` given as a `&T`), the position
    /// is further in the argument's own type.
    fn arg_values_at(
        &mut self,
        j: usize,
        v: &Val,
        t: &Type,
        pos: &[usize],
    ) -> Vec<(Vec<Formula>, Cand)> {
        if let Some(Some((n, actual))) = self.coerced_args.get(j).cloned()
            && pos.first() == Some(&0)
        {
            let mut p = vec![0; n + 1];
            p.extend_from_slice(&pos[1..]);
            return self.values_at(v, &actual, &p);
        }
        self.values_at(v, t, pos)
    }

    /// Checks each closure or function a call gives for a type parameter
    /// `F` the callee calls (`F::SIG = fn(A, ..) -> R`): where what it may
    /// give `F` in an `A` has something declared behind a `&mut` (or in an
    /// `Unsafe`), `F` must be a closure walked with exactly that of its
    /// parameter (`Walk::closure_decls`), so that what it writes there keeps
    /// it, and what it counts on there holds (lemma W7 of
    /// docs/63-what-is-declared.md). A closure written in the call that
    /// the callee doesn't keep is (`closure_trees`); one made before, or
    /// kept to be called later, or a function, says nothing of its
    /// parameters, so it can't be given one.
    #[allow(clippy::too_many_arguments)]
    fn closures_given(
        &mut self,
        s: &crate::types::Scheme,
        fsig: &crate::types::FnType,
        insts: &[(String, Inst)],
        subst: &crate::types::Subst,
        spans: &[Span],
        at: Span,
        fname: &str,
    ) {
        for (a, b) in &s.eqs {
            let (Type::Proj(base, _, n), Type::Fn(sig)) = (a, b) else {
                continue;
            };
            let Type::Param(f, _) = &**base else {
                continue;
            };
            if n != crate::traits::SIG {
                continue;
            }
            let fty = self.cx.solved(&subst.apply(base));
            let closure = match &fty {
                Type::Adt(symbol, ..) if self.cx.traits.closures.contains_key(symbol) => {
                    Some(symbol.clone())
                }
                _ => None,
            };
            if closure
                .as_ref()
                .is_some_and(|c| self.closures_refused.contains(c))
            {
                continue;
            }
            let have: Vec<Tree> = (closure.as_ref())
                .and_then(|c| self.closure_decls.get(c).cloned())
                .unwrap_or_default();
            let span = (fsig.params.iter().position(|p| free::mentions(p, f)))
                .and_then(|j| spans.get(j).copied())
                .unwrap_or(at);
            for (i, p) in sig.params.iter().enumerate() {
                let want = self.subst_tree(p, insts, subst, 0);
                let pt = self.cx.solved(&subst.apply(p));
                let have_i = have.get(i).cloned().unwrap_or_default();
                for (pos, d) in &want {
                    if d.is_any() {
                        continue;
                    }
                    let Some((t, Variance::Inv)) = self.followed(&pt, pos, span) else {
                        continue;
                    };
                    let here: Vec<&Disj> = (have_i.iter())
                        .filter(|(q, _)| q == pos)
                        .map(|(_, h)| h)
                        .collect();
                    let cases = match here.is_empty() {
                        true => vec![(Vec::new(), Cand::Any)],
                        false => vec![(
                            Vec::new(),
                            Cand::all(here.iter().map(|h| (*h).clone()).collect()),
                        )],
                    };
                    let writes = self.implies(&cases, d, &t).is_ok();
                    let given = [(Vec::new(), Cand::all(vec![d.clone()]))];
                    let reads = here.iter().all(|h| self.implies(&given, h, &t).is_ok());
                    if !(writes && reads) {
                        let text = d.text().unwrap_or_default();
                        self.failed(Error::ty(
                            span,
                            format!(
                                "can't prove `{text}` of what this may write: `{fname}` gives \
                                 it a `&mut` to what's declared so, and only a closure written \
                                 in the call, which `{fname}` doesn't keep, is checked with what \
                                 it's given"
                            ),
                        ));
                        if let Some(c) = closure {
                            self.closures_refused.insert(c);
                        }
                        return;
                    }
                }
            }
        }
    }

    /// `tree`, of argument `j`'s own type, as of the parameter's: the
    /// positions a coercion looked through left out.
    fn arg_tree(&self, j: usize, tree: Tree) -> Tree {
        match self.coerced_args.get(j) {
            Some(Some((n, _))) => (tree.into_iter())
                .filter_map(|(p, d)| {
                    let n = *n;
                    if p.len() > n && p[..=n].iter().all(|k| *k == 0) {
                        let mut q = vec![0];
                        q.extend_from_slice(&p[n + 1..]);
                        Some((q, d))
                    } else {
                        None
                    }
                })
                .collect(),
            _ => tree,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn call_trees_now(
        &mut self,
        symbol: &str,
        targs: &[Type],
        sig: Option<&RefinedSig>,
        vals: &[Val],
        places: &[Option<Place>],
        spans: &[Span],
        at: Span,
    ) -> Result<CallTrees, Error> {
        let known = self.cx.fns.iter().any(|f| f.symbol == symbol)
            || (self.cx.traits.traits.values())
                .any(|t| t.methods.iter().any(|m| m.symbol == symbol));
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
            None => (
                symbol.rsplit("::").next().unwrap_or(symbol).to_string(),
                Vec::new(),
            ),
        };
        let pname = |j: usize| match pnames.get(j) {
            Some(n) => n.clone(),
            None => format!("{}", j + 1),
        };
        // What's declared of each place lent `&mut`, as of the argument.
        let mut written: Vec<Option<Tree>> = Vec::new();
        for (j, p) in places.iter().take(vals.len()).enumerate() {
            let w = p.as_ref().map(|p| under(&self.place_decl(p), 0));
            written.push(w.map(|w| self.arg_tree(j, w)));
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
            match self.coerced_args.get(j).cloned().flatten() {
                Some((n, actual)) => {
                    let mut moved = Vec::new();
                    for (p, d) in tree {
                        if p.first() == Some(&0) {
                            let mut q = vec![0; n + 1];
                            q.extend_from_slice(&p[1..]);
                            moved.push((q, d.clone()));
                        }
                    }
                    self.need_tree(v, &actual, &moved, None, span, &what)?;
                }
                None => self.need_tree(v, t, tree, None, span, &what)?,
            }
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
        let behind_text = |text: &str, j: usize| {
            format!(
                "can't prove `{text}` of what `{fname}` may write behind argument `{}`",
                pname(j)
            )
        };
        let arg_text = |text: &str, j: usize| {
            format!(
                "can't prove `{text}` for argument `{}` of `{fname}`",
                pname(j)
            )
        };
        self.check_behind(&s, &subst, &insts, spans, at, &behind_text, &arg_text);
        // Where a parameter's type is invariant but not as a type
        // parameter's, what's written there must be exactly what's declared
        // of the argument there: what the callee may write is what its own
        // type says.
        let scoped = self.cx.scoped_params(symbol);
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
                None => {
                    let w = self.arg_written(j, &v, &t);
                    self.arg_tree(j, w)
                }
            };
            for (pos, want) in &decl {
                if want.is_any() || inside.iter().any(|p| pos.starts_with(p)) {
                    continue;
                }
                // What a place lent straight to a call-scoped `&mut` is
                // declared to be itself is proved again after the call, of
                // what the call left there, as its postconditions say
                // (`Walk::post`): nothing reads it until then.
                if pos.as_slice() == [0]
                    && places.get(j).is_some_and(Option::is_some)
                    && scoped.get(j) == Some(&true)
                {
                    continue;
                }
                let Some((pt, Variance::Inv)) =
                    self.followed(&t, pos, spans.get(j).copied().unwrap_or(at))
                else {
                    continue;
                };
                let have: Vec<(Vec<Formula>, Cand)> = (param_trees[j].iter())
                    .filter(|(p, _)| p == pos)
                    .map(|(_, d)| (Vec::new(), Cand::all(vec![d.clone()])))
                    .collect();
                let have = match have.is_empty() {
                    true => vec![(Vec::new(), Cand::Any)],
                    false => have,
                };
                if self.implies(&have, want, &pt).is_err() {
                    let text = want.text().unwrap_or_default();
                    let span = spans.get(j).copied().unwrap_or(at);
                    self.failed(Error::ty(
                        span,
                        format!(
                            "can't prove `{text}` of what `{fname}` may write behind argument `{}`",
                            pname(j)
                        ),
                    ));
                }
            }
        }
        let plain: Vec<(String, Inst)> = (insts.iter())
            .map(|(x, inst, _)| (x.clone(), inst.clone()))
            .collect();
        self.closures_given(&s, fsig, &plain, &subst, spans, at, &fname);
        // What it gives, and leaves behind what it's lent: of each type
        // parameter, its instantiation; of each projection on one, what
        // the impl says it is (lemma W6 of docs/63-what-is-declared.md).
        let mut out = CallTrees::default();
        let ret = (*fsig.ret).clone();
        let mut unknown = Vec::new();
        out.result = self.subst_tree_unknown(&ret, &plain, &subst, 0, &mut unknown);
        // Where what's declared isn't known, nothing may be written: a
        // `&mut` there, or an `Unsafe`, would say nothing's declared.
        let rt = self.cx.solved(&subst.apply(&ret));
        for (rho, why) in unknown {
            let Some((t, _)) = self.type_at(&rt, &rho) else {
                continue;
            };
            let writable = (self.positions(&t).iter())
                .any(|p| matches!(self.type_at(&t, p), Some((_, Variance::Inv))));
            if writable {
                self.failed(Error::ty(
                    at,
                    format!(
                        "can't tell what's declared behind what `{fname}` gives: it's {why}, \
                         which what it's of doesn't say"
                    ),
                ));
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
    /// Checks what each type parameter's instantiation at a call, of a
    /// function of scheme `s` at type arguments `subst`, needs of what's
    /// behind each `&mut` (lemmas A3, A5 and W5): that every case of what
    /// the callee may write there has what's declared there. Messages say
    /// `behind_text` of what's written behind argument `j`, `arg_text` of
    /// an argument's value that doesn't have it.
    #[allow(clippy::too_many_arguments)]
    fn check_behind(
        &mut self,
        s: &crate::types::Scheme,
        subst: &crate::types::Subst,
        insts: &[(String, Inst, Vec<Behind>)],
        spans: &[Span],
        at: Span,
        behind_text: &dyn Fn(&str, usize) -> String,
        arg_text: &dyn Fn(&str, usize) -> String,
    ) {
        for (x, inst, behind) in insts {
            let xt = self.cx.solved(&subst.apply(&s.params.ty(x)));
            for b in behind {
                for (pos, want) in &b.declared {
                    if want.is_any() {
                        continue;
                    }
                    let Some((pt, _)) =
                        self.followed(&xt, pos, spans.get(b.arg).copied().unwrap_or(at))
                    else {
                        continue;
                    };
                    let span = spans.get(b.arg).copied().unwrap_or(at);
                    let text = want.text().unwrap_or_default();
                    let behind_msg = behind_text(&text, b.arg);
                    if let Some(why) = inst.unknown_at(pos) {
                        self.failed(Error::ty(
                            span,
                            format!("{behind_msg}: {why} can make or change a value of it"),
                        ));
                        continue;
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
                                false => arg_text(&text, src.arg),
                            };
                            self.failed(Error::ty(span, msg));
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Checks `v`, the result of function `name`, whose type `ret` has
    /// opaque types in it, as what's given to code generic in each, which
    /// knows of it only what its bounds say: the rest of the program
    /// (lemma A10 of docs/68-fail-closed.md). So what's declared behind a
    /// `&mut` in the type each stands for, or in an `Unsafe`, must be what
    /// that code may write there: of what it's given, unless a bound can
    /// make or change a value there (`makes`, `changes`), as an equality
    /// bound `Item = &'a mut usize` can.
    pub(super) fn opaque_result(&mut self, v: &Val, ret: &Type, span: Span, name: &str) {
        let symbols = crate::check::opaque::opaques_of(ret);
        if symbols.is_empty() {
            return;
        }
        let mut names: Vec<String> = Vec::new();
        let mut hidden: Vec<Type> = Vec::new();
        let mut defs = Vec::new();
        for s in &symbols {
            let def = self.cx.traits.opaques[s].clone();
            let Some(h) = def.hidden.clone() else {
                // Fail-closed: what it stands for isn't known.
                self.failed(Error::ty(
                    span,
                    format!(
                        "can't tell what the result of `{name}` is, to check what's declared in it"
                    ),
                ));
                return;
            };
            // The first is `Self` in messages, as it would be in its
            // trait: `Self::Item = &'a mut usize`.
            let mut x = match names.is_empty() {
                true => "Self".to_string(),
                false => crate::types::opaque_text(s).to_string(),
            };
            while names.contains(&x) {
                x.push('\'');
            }
            names.push(x);
            hidden.push(self.cx.solved(&h));
            defs.push((s.clone(), def));
        }
        let params = crate::types::Params::new(names.clone(), Vec::new());
        let to_param = |t: &Type| -> Type {
            fn go(
                t: &Type,
                symbols: &[String],
                params: &crate::types::Params,
                names: &[String],
            ) -> Type {
                if let Type::Opaque(s, ..) = t
                    && let Some(k) = symbols.iter().position(|o| o == s)
                {
                    return params.ty(&names[k]);
                }
                let mut parts = t
                    .parts()
                    .map(|p| go(p, symbols, params, names))
                    .collect::<Vec<_>>()
                    .into_iter();
                t.map(|_| parts.next().expect("as many"))
            }
            go(t, &symbols, &params, &names)
        };
        let mut preds = Vec::new();
        let mut eqs = Vec::new();
        for (_, def) in &defs {
            for p in &def.preds {
                preds.push(crate::traits::Pred {
                    ty: to_param(&p.ty),
                    trait_name: p.trait_name.clone(),
                });
            }
            for (a, b) in &def.eqs {
                eqs.push((to_param(a), to_param(b)));
            }
        }
        let scheme = crate::types::Scheme {
            params: params.clone(),
            lifetimes: Vec::new(),
            bounds: Vec::new(),
            preds,
            eqs,
            ty: Type::func(vec![to_param(ret)], Type::Unit),
        };
        let subst = crate::types::Subst::new(&names, &hidden);
        let vals = [v.clone()];
        let written = [None];
        let mut insts = Vec::new();
        for x in &names {
            let mut behind = Vec::new();
            let inst = self.instantiate_one(&scheme, x, &subst, &vals, &written, &mut behind);
            insts.push((x.clone(), inst, behind));
        }
        let behind_text = |text: &str, _: usize| {
            format!(
                "can't prove `{text}` of what may be written through the result of `{name}`, `{ret}`"
            )
        };
        let arg_text =
            |text: &str, _: usize| format!("can't prove `{text}` of the result of `{name}`");
        self.check_behind(
            &scheme,
            &subst,
            &insts,
            &[span],
            span,
            &behind_text,
            &arg_text,
        );
    }
}

impl Walk<'_> {
    /// Says of array `v`, of type `t`, that each of its elements is one of
    /// `items`, at each of their positions.
    pub(super) fn elements(&mut self, v: &Val, t: &Type, items: &[Val], spans: &[Span], at: Span) {
        let Type::Array(elem, _) = t else {
            return;
        };
        let elem = self.cx.solved(elem);
        let held: Vec<Held> = (items.iter().enumerate())
            .map(|(i, item)| Held {
                val: item,
                ty: elem.clone(),
                at: Vec::new(),
                span: spans.get(i).copied().unwrap_or(at),
                what: format!("element {i} of the array"),
            })
            .collect();
        self.agree_inv(&held);
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

/// One of the values a value made here holds at a type parameter's
/// positions (`Walk::agree_inv`): the value, its type, where the parameter
/// is in it, where it's written, and what it is, for messages.
pub(super) struct Held<'v> {
    pub(super) val: &'v Val,
    pub(super) ty: Type,
    pub(super) at: Pos,
    pub(super) span: Span,
    pub(super) what: String,
}

impl Walk<'_> {
    /// Checks that `srcs`, the values a value made here holds at one of
    /// its type parameter's positions, agree on what's declared where it's
    /// invariant (behind a `&mut`, in an `Unsafe`): what's written through
    /// one may be written through another, as the value's type says they're
    /// of one type (lemma A8 of docs/68-fail-closed.md). A struct's two
    /// `&mut T` fields, an array's `&mut T` elements.
    pub(super) fn agree_inv(&mut self, srcs: &[Held]) {
        if srcs.len() < 2 {
            return;
        }
        // Of each source, at each position inside the parameter where it's
        // invariant, what's declared there.
        let mut decls: Vec<(usize, Pos, Type, Vec<Disj>)> = Vec::new();
        for (i, src) in srcs.iter().enumerate() {
            let Some((xt, _)) = self.type_at(&src.ty, &src.at) else {
                continue;
            };
            let written = self.written_tree(src.val, &src.ty);
            let mut qs = vec![Vec::new()];
            qs.extend(self.positions(&xt));
            for q in qs {
                let mut p = src.at.clone();
                p.extend_from_slice(&q);
                let Some((pt, Variance::Inv)) = self.followed(&src.ty, &p, src.span) else {
                    continue;
                };
                let here: Vec<Disj> = (written.iter())
                    .filter(|(r, d)| *r == p && !d.is_any())
                    .map(|(_, d)| d.clone())
                    .collect();
                decls.push((i, q, pt, here));
            }
        }
        for (i, q, pt, have) in &decls {
            for (j, r, _, want) in &decls {
                if i == j || q != r {
                    continue;
                }
                let have = match have.is_empty() {
                    true => vec![(Vec::new(), Cand::Any)],
                    false => vec![(Vec::new(), Cand::all(have.clone()))],
                };
                if let Some(d) = (want.iter()).find(|d| self.implies(&have, d, pt).is_err()) {
                    let text = d.text().unwrap_or_default();
                    let (mine, theirs) = (&srcs[*i], &srcs[*j]);
                    let msg = match mine.what == theirs.what {
                        true => format!(
                            "can't prove `{text}` of what may be written through {}: it's of one \
                             type in two places, and one has it",
                            mine.what
                        ),
                        false => format!(
                            "can't prove `{text}` of what may be written through {}: it's of the \
                             same type as {}, which has it",
                            mine.what, theirs.what
                        ),
                    };
                    self.failed(Error::ty(mine.span, msg));
                    return;
                }
            }
        }
    }

    /// Does `tree`, of a value of type `t`, say anything where `t` is
    /// invariant?
    pub(super) fn declared_inv(&mut self, t: &Type, tree: &Tree) -> bool {
        for (pos, d) in tree {
            if d.is_any() {
                continue;
            }
            if let Some((_, Variance::Inv)) = self.type_at(t, pos) {
                return true;
            }
        }
        false
    }
}

/// The object value `v` is, if it has one.
pub(super) fn obj_of(v: &Val) -> Option<Obj> {
    match v {
        Val::Struct(o, _) | Val::Ref(o, _) | Val::Obj(o) | Val::Array(o, _) => Some(*o),
        _ => None,
    }
}

impl Walk<'_> {
    /// Object `o`, of type `t`, is, where branches meet, branch `i`'s
    /// object `each[i]` where `guards[i]` holds: at each position, what's
    /// known of that one's values there, under that condition. Of an
    /// enum's, that's what's said of it and of its variants' fields, each
    /// where it's that variant: so `None` has no values in its `Some`, and
    /// joined with a `Some`, what's known is that one's.
    pub(super) fn join_trees(
        &mut self,
        o: Obj,
        t: &Type,
        each: &[Option<Obj>],
        guards: &[Formula],
    ) {
        let enums = each
            .iter()
            .all(|b| b.is_some_and(|b| self.parts.contains_key(&b)));
        let mut positions: Vec<Pos> = Vec::new();
        if enums {
            positions = self.positions(t);
        } else {
            for b in each.iter().flatten() {
                for (p, _) in self.trees.get(b).into_iter().flatten() {
                    if !positions.contains(p) {
                        positions.push(p.clone());
                    }
                }
            }
        }
        if positions.is_empty() {
            return;
        }
        let was = std::mem::replace(&mut self.no_place_reads, true);
        let mut tree = Vec::new();
        for pos in positions {
            let mut cases = Vec::new();
            for (b, g) in each.iter().zip(guards) {
                let here = match b {
                    Some(b) if enums => self.values_at(&Val::Obj(*b), t, &pos),
                    _ => {
                        let reg = self.registered(*b, &pos);
                        match reg.is_empty() {
                            true => vec![(Vec::new(), Cand::Any)],
                            false => vec![(Vec::new(), Cand::all(reg))],
                        }
                    }
                };
                for (mut cg, c) in here {
                    cg.insert(0, g.clone());
                    cases.push((cg, c));
                }
            }
            let d = Disj(cases);
            if !d.is_any() {
                tree.push((pos, d));
            }
        }
        self.no_place_reads = was;
        match enums {
            // Of its variants' fields too, as a pattern reads them.
            true => self.assume_tree(&Val::Obj(o), t, &tree),
            false => self.trees.entry(o).or_default().extend(tree),
        }
    }
}

impl Walk<'_> {
    /// What may be written at the positions of argument `j`, `v`, in its
    /// own type (`written_tree`), coerced or not.
    fn arg_written(&mut self, j: usize, v: &Val, t: &Type) -> Tree {
        match self.coerced_args.get(j).cloned().flatten() {
            Some((_, actual)) => self.written_tree(v, &actual),
            None => self.written_tree(v, t),
        }
    }
}

impl Walk<'_> {
    /// Checks that what may be written at each invariant position of `v`,
    /// given as a value of type `t` of which `declared` is what's declared,
    /// is declared there too (lemma A4): or else what's given it could
    /// write past it. Not under the positions `skip`, of the type's own
    /// type parameters, which say it themselves.
    pub(super) fn keep_inv(
        &mut self,
        v: &Val,
        t: &Type,
        declared: &Tree,
        skip: &[Pos],
        span: Span,
        what: &str,
    ) -> Result<(), Error> {
        let written = self.written_tree(v, t);
        for (pos, want) in &written {
            if want.is_any() || skip.iter().any(|s| pos.starts_with(s)) {
                continue;
            }
            let Some((pt, Variance::Inv)) = self.followed(t, pos, span) else {
                continue;
            };
            let have: Vec<(Vec<Formula>, Cand)> = (declared.iter())
                .filter(|(p, _)| p == pos)
                .map(|(_, d)| (Vec::new(), Cand::all(vec![d.clone()])))
                .collect();
            let have = match have.is_empty() {
                true => vec![(Vec::new(), Cand::Any)],
                false => have,
            };
            if self.implies(&have, want, &pt).is_err() {
                let text = want.text().unwrap_or_default();
                self.failed(Error::ty(
                    span,
                    format!("can't prove `{text}` of what may be written through {what}"),
                ));
            }
        }
        Ok(())
    }

    /// The positions in `generic`, a field's type as declared, of the
    /// type parameters `params` of what it's a field of.
    pub(super) fn param_positions(&mut self, generic: &Type, params: &[String]) -> Vec<Pos> {
        let mut out = Vec::new();
        for x in params {
            let (occ, _) = self.occurrences(generic, x);
            out.extend(occ.into_iter().map(|(p, _)| p));
        }
        out
    }

    /// Checks the fields `vals` of a value of struct or enum `t` (variant
    /// `variant`), made here, as `keep_inv` says, of each field's type but
    /// where it's a type parameter of `t`'s.
    pub(super) fn keep_fields_inv(
        &mut self,
        t: &Type,
        variant: Option<usize>,
        vals: &[Val],
        names: &[String],
        span: &[Span],
    ) -> Result<(), Error> {
        let Type::Adt(symbol, ..) = t else {
            return Ok(());
        };
        let Some(def) = self.cx.adts.get(symbol).cloned() else {
            return Ok(());
        };
        let params = def.params.as_slice().to_vec();
        let (generic, inst): (Vec<Type>, Vec<Type>) = match (&def.kind, variant) {
            (crate::types::AdtKind::Struct(fields), None) => (
                fields.iter().map(|(_, t)| t.clone()).collect(),
                struct_fields(&self.cx.adts, t)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(_, t)| t)
                    .collect(),
            ),
            (crate::types::AdtKind::Enum(vs), Some(v)) => match vs.get(v) {
                Some((_, ts)) => (
                    ts.clone(),
                    crate::types::variant_fields(&self.cx.adts, t, v),
                ),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let name = symbol.rsplit("::").next().unwrap_or(symbol).to_string();
        let field_what = |j: usize| match names.get(j) {
            Some(n) => format!("field `{n}` of `{name}`"),
            None => format!("field {j} of `{name}`"),
        };
        for k in 0..params.len() {
            let held: Vec<Held> = (self.fields_with(t, k).into_iter())
                .filter(|(w, ..)| variant.is_none_or(|v| v == *w))
                .filter_map(|(_, f, rho, ft)| {
                    Some(Held {
                        val: vals.get(f)?,
                        ty: ft,
                        at: rho,
                        span: span.get(f).copied().unwrap_or(span[0]),
                        what: field_what(f),
                    })
                })
                .collect();
            self.agree_inv(&held);
        }
        for (j, v) in vals.iter().enumerate() {
            let (Some(g), Some(ft)) = (generic.get(j), inst.get(j)) else {
                continue;
            };
            let ft = self.cx.solved(ft);
            let skip = self.param_positions(g, &params);
            let declared = match (variant, self.refined_fields(t).cloned()) {
                (None, Some(fields)) => match fields.refs.get(j).cloned().flatten() {
                    Some(r) if r.has_carried() => {
                        let args = self.adt_args(t);
                        self.carried_tree(&r, Env::Fields(vals, &args))
                    }
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            };
            let field = names.get(j).cloned().unwrap_or_else(|| j.to_string());
            let what = format!("field `{field}` of `{name}`");
            let at = span.get(j).copied().unwrap_or_else(|| span[0]);
            self.keep_inv(v, &ft, &declared, &skip, at, &what)?;
        }
        Ok(())
    }
}

impl Walk<'_> {
    /// The positions of each projection in `t`, a type as declared, with
    /// the projection.
    fn projections(&mut self, t: &Type) -> Vec<(Pos, Type)> {
        fn go(w: &mut Walk, t: &Type, at: &mut Pos, out: &mut Vec<(Pos, Type)>) {
            if let Type::Proj(..) = t {
                out.push((at.clone(), t.clone()));
                return;
            }
            if at.len() >= DEPTH {
                return;
            }
            for k in 0..Walk::arity(t) {
                let Some((c, _)) = w.child_declared(t, k) else {
                    continue;
                };
                at.push(k);
                go(w, &c, at, out);
                at.pop();
            }
        }
        let mut out = Vec::new();
        go(self, t, &mut Vec::new(), &mut out);
        out
    }

    /// What `insts`, each type parameter's instantiation, say of a value of
    /// type `a` (as declared in a scheme, whose type arguments are
    /// `subst`): at each position a parameter is at, what it is; and
    /// through a projection on one, `<X as Tr>::A`, what the impl `select`
    /// finds for what `X` is there says `A` is, of what's at the positions
    /// of the impl's type parameters in `X` (lemma A7).
    pub(super) fn subst_tree(
        &mut self,
        a: &Type,
        insts: &[(String, Inst)],
        subst: &crate::types::Subst,
        depth: usize,
    ) -> Tree {
        self.subst_tree_unknown(a, insts, subst, depth, &mut Vec::new())
    }

    /// `subst_tree`, and in `unknown` each position of `a` that's a
    /// projection whose values it can't say, though something is said of
    /// what it's a projection of, with why (lemma W6 of
    /// docs/63-what-is-declared.md): what's declared there isn't known,
    /// so it mustn't be taken to be nothing. An impl's type parameter is
    /// what's at its position in `X` only if it has one position there, or
    /// each says the same; one that isn't in `X` at all (`Map`'s `B`, which
    /// a bound decides) isn't anything `X` says.
    pub(super) fn subst_tree_unknown(
        &mut self,
        a: &Type,
        insts: &[(String, Inst)],
        subst: &crate::types::Subst,
        depth: usize,
        unknown: &mut Vec<(Pos, String)>,
    ) -> Tree {
        let mut out = Vec::new();
        if depth > DEPTH {
            return out;
        }
        for (x, inst) in insts {
            if inst.any.is_some() {
                continue;
            }
            let (occ, _) = self.occurrences(a, x);
            for (rho, _) in occ {
                for (pos, d) in inst.tree() {
                    let mut p = rho.clone();
                    p.extend(pos);
                    out.push((p, d));
                }
            }
        }
        for (rho, proj) in self.projections(a) {
            let Type::Proj(base, tr, name) = &proj else {
                continue;
            };
            // What's said of the parameters the projection is of.
            let said = |insts: &[(String, Inst)]| {
                (insts.iter()).any(|(y, i)| free::mentions(base, y) && !i.tree().is_empty())
            };
            let Type::Param(x, _) = &**base else {
                if said(insts) {
                    unknown.push((rho, format!("`{proj}`, of a projection")));
                }
                continue;
            };
            let Some((_, inst)) = insts.iter().find(|(y, _)| y == x) else {
                continue;
            };
            if inst.any.is_some() || inst.tree().is_empty() {
                continue;
            }
            let base_ty = self.cx.solved(&subst.apply(base));
            let Some((i, impl_args)) = self.cx.traits.select(&base_ty, Some(tr), &[]) else {
                unknown.push((rho, format!("`{proj}`, which no impl says")));
                continue;
            };
            let def = self.cx.traits.impls[i].clone();
            let Some((_, assoc)) = def.assoc.iter().find(|(n, _)| n == name) else {
                unknown.push((rho, format!("`{proj}`, which no impl says")));
                continue;
            };
            let names = def.params.as_slice().to_vec();
            if names.len() != impl_args.len() {
                unknown.push((rho, format!("`{proj}`")));
                continue;
            }
            // Each impl parameter is what's at its position in `X`; but not
            // one a bound of the impl's can make values of, as for a call's
            // (`makes`): its methods could give those.
            let as_scheme = crate::types::Scheme {
                params: def.params.clone(),
                lifetimes: def.lifetimes.clone(),
                bounds: Vec::new(),
                preds: def.preds.clone(),
                eqs: def.eqs.clone(),
                ty: Type::Unit,
            };
            let mut subs = Vec::new();
            let mut unsaid = Vec::new();
            for (k, p) in names.iter().enumerate() {
                if !def.params.is_type(p) {
                    continue;
                }
                let arg = impl_args.get(k).cloned();
                if makes(self.cx, &as_scheme, p, arg.as_ref()).is_some() {
                    unsaid.push(p.clone());
                    continue;
                }
                // What the impl's bounds can change inside it (`changes`),
                // and what the call's could.
                let cx = self.cx;
                let at_impl = |t: &Type| {
                    let s = crate::types::Subst::new(&names, &impl_args);
                    Some(cx.solved(&s.apply(t)))
                };
                let own = match (changes(cx, &as_scheme, p, &at_impl), &arg) {
                    (Some(why), Some(a)) => Some((why, self.inv_positions(a))),
                    _ => None,
                };
                let (occ, _) = self.occurrences(&def.self_ty, p);
                let mut each = (occ.into_iter()).map(|(sigma, _)| {
                    let at: Vec<(Pos, Vec<Case>)> = (inst.at.iter())
                        .filter(|(q, _)| q.starts_with(&sigma))
                        .map(|(q, c)| (q[sigma.len()..].to_vec(), c.clone()))
                        .collect();
                    let mut changed = own.clone();
                    if let Some((why, inv)) = &inst.changed {
                        let (_, mine) = changed.get_or_insert_with(|| (why.clone(), Vec::new()));
                        mine.extend(
                            (inv.iter())
                                .filter(|q| q.starts_with(&sigma))
                                .map(|q| q[sigma.len()..].to_vec()),
                        );
                    }
                    Inst {
                        at,
                        any: None,
                        changed,
                    }
                });
                let Some(first) = each.next() else {
                    unsaid.push(p.clone());
                    continue;
                };
                let tree = first.tree();
                if each.any(|other| other.tree() != tree) {
                    unsaid.push(p.clone());
                    continue;
                }
                subs.push((p.clone(), first));
            }
            // A parameter nothing in `X` says is unknown where it's in `A`.
            for p in &unsaid {
                if free::mentions(assoc, p) {
                    unknown.push((rho.clone(), format!("`{proj}`, which is `{assoc}`")));
                }
            }
            let impl_subst = crate::types::Subst::new(&names, &impl_args);
            let mut inner = Vec::new();
            for (q, d) in self.subst_tree_unknown(assoc, &subs, &impl_subst, depth + 1, &mut inner)
            {
                let mut p = rho.clone();
                p.extend(q);
                out.push((p, d));
            }
            for (q, why) in inner {
                let mut p = rho.clone();
                p.extend(q);
                unknown.push((p, why));
            }
        }
        out
    }

    /// What each closure literal given to a call of function `symbol`
    /// (resolved, with type arguments `targs`) can count on of each of its
    /// parameters, by its argument's index: of a type parameter no closure
    /// gives back, nor a bound can make (`makes`), what the call's other
    /// arguments, `vals`, have of it (lemma A2: generic code can only give
    /// it those).
    pub(super) fn closure_trees(
        &mut self,
        symbol: &str,
        targs: &[Type],
        vals: &[Val],
        places: &[Option<Place>],
        closures: &[usize],
    ) -> HashMap<usize, Vec<Tree>> {
        let mut out = HashMap::new();
        let known = self.cx.fns.iter().any(|f| f.symbol == symbol)
            || (self.cx.traits.traits.values())
                .any(|t| t.methods.iter().any(|m| m.symbol == symbol));
        if !known || closures.is_empty() {
            return out;
        }
        let s = self.cx.scheme(symbol).clone();
        let Type::Fn(fsig) = &s.ty else {
            return out;
        };
        let names = s.params.as_slice().to_vec();
        if names.len() != targs.len() {
            return out;
        }
        let subst = crate::types::Subst::new(&names, targs);
        let mut written: Vec<Option<Tree>> = Vec::new();
        for p in places.iter().take(vals.len()) {
            written.push(p.as_ref().map(|p| under(&self.place_decl(p), 0)));
        }
        written.resize(vals.len(), None);
        let mut insts = Vec::new();
        for x in &names {
            if !s.params.is_type(x) {
                continue;
            }
            let mut behind = Vec::new();
            let inst = self.instantiate_one(&s, x, &subst, vals, &written, &mut behind);
            insts.push((x.clone(), inst));
        }
        fn has_opaque(t: &Type) -> bool {
            matches!(t, Type::Opaque(..)) || t.parts().any(has_opaque)
        }
        for &j in closures {
            let Some(Type::Param(f, _)) = fsig.params.get(j) else {
                continue;
            };
            // Only one the callee can't give back, or keep anywhere the
            // caller can call it from after: its type nowhere else in the
            // signature. Else it could be called with anything later.
            let elsewhere = (fsig.params.iter().enumerate())
                .any(|(i, p)| i != j && free::mentions(p, f))
                || free::mentions(&fsig.ret, f)
                || has_opaque(&fsig.ret);
            if elsewhere {
                continue;
            }
            let sig = (s.eqs.iter()).find_map(|(a, b)| match (a, b) {
                (Type::Proj(base, _, n), Type::Fn(sig))
                    if n == crate::traits::SIG
                        && matches!(&**base, Type::Param(g, _) if g == f) =>
                {
                    Some(sig.clone())
                }
                _ => None,
            });
            let Some(sig) = sig else {
                continue;
            };
            let trees: Vec<Tree> = (sig.params.iter())
                .map(|p| self.subst_tree(p, &insts, &subst, 0))
                .collect();
            if trees.iter().any(|t| !t.is_empty()) {
                out.insert(j, trees);
            }
        }
        out
    }
}

/// The way from a value of type `t` to each value at position `pos` of it,
/// for the interpreter to check what's said of them (`Measures::reach`):
/// through each field the type parameter there is in, as its type has it.
pub(super) fn reach(adts: &crate::types::Adts, t: &Type, pos: &[usize]) -> Vec<crate::ast::Reach> {
    use crate::ast::{Reach, ReachStep};
    fn node(
        adts: &crate::types::Adts,
        t: &Type,
        pos: &[usize],
        memo: &mut HashMap<(Type, Pos), usize>,
        out: &mut Vec<Reach>,
    ) -> usize {
        let key = (t.clone(), pos.to_vec());
        if let Some(&i) = memo.get(&key) {
            return i;
        }
        let i = out.len();
        out.push(Reach::default());
        memo.insert(key, i);
        if out.len() > 256 {
            return i;
        }
        let Some((&k, rest)) = pos.split_first() else {
            out[i].here = true;
            return i;
        };
        let mut steps = Vec::new();
        let mut go = |step: ReachStep, t: &Type, p: Vec<usize>, out: &mut Vec<Reach>| {
            let n = node(adts, t, &p, memo, out);
            steps.push((step, n));
        };
        match t {
            Type::Tuple(ts) => {
                if let Some(e) = ts.get(k) {
                    go(ReachStep::Field(k.to_string()), e, rest.to_vec(), out);
                }
            }
            Type::Box(inner) | Type::Ref(_, _, inner) if k == 0 => {
                go(ReachStep::Deref, inner, rest.to_vec(), out);
            }
            Type::Slice(e) | Type::Array(e, _) if k == 0 => {
                go(ReachStep::Elems, e, rest.to_vec(), out);
            }
            Type::Adt(symbol, _, args) if crate::intrinsic::is_intrinsic_type(symbol) => {
                if let Some(a) = args.get(k) {
                    let step = match symbol.rsplit("::").next() {
                        Some("RawBuf") => Some(ReachStep::Slots),
                        Some("Slots") => Some(ReachStep::Arena),
                        _ => None,
                    };
                    if let Some(step) = step {
                        go(step, a, rest.to_vec(), out);
                    }
                }
            }
            Type::Adt(symbol, ..) => {
                let Some(def) = adts.get(symbol) else {
                    return i;
                };
                let Some(param) = def.params.as_slice().get(k).cloned() else {
                    return i;
                };
                #[allow(clippy::type_complexity)]
                let variants: Vec<(
                    Option<String>,
                    Vec<(String, Type)>,
                    Vec<Type>,
                )> = match &def.kind {
                    crate::types::AdtKind::Struct(fields) => vec![(
                        None,
                        fields.clone(),
                        struct_fields(adts, t)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(_, t)| t)
                            .collect(),
                    )],
                    crate::types::AdtKind::Enum(vs) => (vs.iter().enumerate())
                        .map(|(v, (name, ts))| {
                            (
                                Some(name.clone()),
                                ts.iter()
                                    .enumerate()
                                    .map(|(j, t)| (j.to_string(), t.clone()))
                                    .collect(),
                                crate::types::variant_fields(adts, t, v),
                            )
                        })
                        .collect(),
                };
                for (variant, generic, inst) in variants {
                    for (j, ((fname, g), ft)) in generic.iter().zip(&inst).enumerate() {
                        for rho in param_paths(g, &param) {
                            let mut p = rho;
                            p.extend_from_slice(rest);
                            let step = match &variant {
                                Some(v) => ReachStep::Variant(v.clone(), j),
                                None => ReachStep::Field(fname.clone()),
                            };
                            go(step, ft, p, out);
                        }
                    }
                }
            }
            _ => {}
        }
        out[i].steps = steps;
        i
    }
    let mut out = Vec::new();
    node(adts, t, pos, &mut HashMap::new(), &mut out);
    out
}

/// The positions of type parameter `x` in `t`, a type as declared.
fn param_paths(t: &Type, x: &str) -> Vec<Pos> {
    fn go(t: &Type, x: &str, at: &mut Pos, out: &mut Vec<Pos>) {
        if matches!(t, Type::Param(p, _) if p == x) {
            out.push(at.clone());
            return;
        }
        if at.len() >= DEPTH || !free::mentions(t, x) {
            return;
        }
        let kids: Vec<&Type> = match t {
            Type::Adt(_, _, args) => args.iter().collect(),
            Type::Box(i) | Type::Ref(_, _, i) | Type::Slice(i) | Type::Array(i, _) => vec![&**i],
            Type::Tuple(ts) => ts.iter().collect(),
            _ => Vec::new(),
        };
        for (k, c) in kids.into_iter().enumerate() {
            at.push(k);
            go(c, x, at, out);
            at.pop();
        }
    }
    let mut out = Vec::new();
    go(t, x, &mut Vec::new(), &mut out);
    out
}

impl Walk<'_> {
    /// `result`, of type `ty`, is, where branches meet, branch `i`'s value
    /// `vals[i]` where `guards[i]` holds: at each position, what's known of
    /// that branch's values there, under that condition. The places as the
    /// branches left them aren't here any more, so what a reference points
    /// to isn't read.
    pub(super) fn join_values(
        &mut self,
        result: &Val,
        ty: &Type,
        vals: &[Val],
        guards: &[Formula],
    ) {
        if obj_of(result).is_none() {
            return;
        }
        let positions = self.positions(ty);
        if positions.is_empty() {
            return;
        }
        let was = std::mem::replace(&mut self.no_place_reads, true);
        let mut tree = Vec::new();
        for pos in positions {
            let mut cases = Vec::new();
            for (v, g) in vals.iter().zip(guards) {
                for (mut cg, c) in self.values_at(v, ty, &pos) {
                    cg.insert(0, g.clone());
                    cases.push((cg, c));
                }
            }
            if cases.iter().all(|(_, c)| c.is_any()) {
                continue;
            }
            tree.push((pos, Disj(cases)));
        }
        self.no_place_reads = was;
        // Of the value, and of what's in its fields, of each variant of an
        // enum's where it's that variant, as a pattern reads them.
        self.assume_tree(result, ty, &tree);
    }
}
