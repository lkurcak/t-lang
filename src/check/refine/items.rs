//! What every value an iterator gives has, its *item invariant*
//! (docs/82-item-invariants.md): `(0..8).rev()` gives only what `0..8`
//! does, values below 8, though `Rev`'s `next` says it gives an
//! `Option<I::Item>`, which says nothing.
//!
//! An iterator's *protocol* is the methods of `Iterator` and
//! `DoubleEndedIterator` that take it by `&mut`: `next`, `next_back`, `any`,
//! `all`. What the walk keeps of an iterator object, its *items*, is a tree
//! of its `Item` type (`targs::Tree`) that holds of every value its protocol
//! gives, as a result or to a closure, after any number of protocol calls
//! from that value (lemma I1). So a protocol call's result has it, and what
//! the call leaves behind the `&mut` does too, as what that gives later is
//! what the first would give later.
//!
//! An object gets its items two ways:
//!
//! - From its type's own signatures (`Walk::envelope`, lemma I2), if its
//!   impls of the two traits define only `next` and `next_back`: the
//!   measures each of those keeps or moves only one way bound every state
//!   it can get to, and what each says of the value it gives, at a state
//!   within those bounds, is what every value has. For `a..b`, that's at
//!   least `a` and less than `b`.
//! - From a call generic in an iterator type parameter that gives an
//!   adapter of it (`Walk::make_items`): by parametricity, code generic in
//!   `X: Iterator` makes no `X` (lemma I3), and an adapter's impl, generic
//!   in its iterator, gets values of `I::Item` only from its iterator's
//!   protocol, and values of a closure's result type only from the closure
//!   (lemma I4). So `Rev<I>` gives what its `I` does; `Enumerate<I>`'s
//!   second parts and `Zip<A, B>`'s parts are what `I`, `A`, `B` give;
//!   `Map<I, F>`'s are what the closure given to `map` gives
//!   (docs/74-refined-instantiation.md).
//!
//! Which parameters of a signature can give a value, and how, is never
//! listed here: every such question is `sources.rs`'s (docs/83-sources.md),
//! which refuses what it doesn't know to be harmless. Each impl of an
//! iterator's protocol is asked, `DoubleEndedIterator`'s too
//! (`Walk::protocol_impls`).

use super::*;
use crate::borrowck::regions::Variance;
use sources::{Of, Shape, Signature, Site};
use std::rc::Rc;
use targs::Pos;
use targs::{Cand, Disj, Tree};

/// The name a loop's frame keeps an iterator's items by, as if it were one
/// of its measures (`Walk::frames_of`): every change to it in the loop is
/// a call of its protocol (lemma I5).
pub(super) const ITEMS: &str = "(items)";

const ITERATOR: &str = crate::check::ITERATOR;

/// `DoubleEndedIterator`, whose `next_back` is of the protocol too.
const ITERATOR_DEI: &str = "core::iter::DoubleEndedIterator";

/// What an iterator's own signatures say of every value it gives (lemma
/// I2): of each of its methods that gives one, its refinements, at type
/// arguments; and of each measure its protocol keeps or moves one way, its
/// value at the object, and which way it may move.
#[derive(Debug)]
pub(super) struct Envelope {
    /// Its type: `Range<usize>`.
    ty: Type,
    /// `next`, and `next_back` if it's double-ended: their names, what
    /// they say, and their impls' type arguments.
    methods: Vec<Method>,
    /// Each measure, its value at the object, and whether it never grows
    /// and never shrinks.
    bounds: Vec<(String, Val, bool, bool)>,
}

/// Which way each measure a protocol call keeps may move: whether it
/// never grows, and whether it never shrinks.
type Moves = Vec<(String, bool, bool)>;

/// A method of an iterator's own that gives a value: its name, what it
/// says, and its impl's type arguments, by name.
type Method = (String, RefinedSig, Vec<(String, Type)>);

/// The same, by its symbol, before what it says is looked up.
type OwnMethod = (String, String, Vec<(String, Type)>);

/// Is `t` a type the impls of which are known: no type parameter,
/// projection, opaque type or variable in it.
fn concrete(t: &Type) -> bool {
    !matches!(
        t,
        Type::Param(..) | Type::Proj(..) | Type::Opaque(..) | Type::Var(_) | Type::Inferred(..)
    ) && t.parts().all(concrete)
}

/// Is `t` `P::Item`, for type parameter `P`?
fn item_of(t: &Type) -> Option<&str> {
    match t {
        Type::Proj(base, tr, name) if name == "Item" && (tr == ITERATOR || tr == ITERATOR_DEI) => {
            match &**base {
                Type::Param(p, _) => Some(p),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Does `t` mention type parameter `x`?
fn mentions_param(t: &Type, x: &str) -> bool {
    matches!(t, Type::Param(p, _) if p == x) || t.parts().any(|s| mentions_param(s, x))
}

/// Where each value in an item of type `item` comes from, by position
/// (`targs::Pos`): the whole item, or a tuple's element, at any depth.
fn sources(item: &Type, at: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, Type)>) {
    match item {
        Type::Tuple(ts) => {
            for (k, t) in ts.iter().enumerate() {
                at.push(k);
                sources(t, at, out);
                at.pop();
            }
        }
        Type::Ref(_, _, t) | Type::Box(t) if !matches!(**t, Type::Param(..)) => {
            at.push(0);
            sources(t, at, out);
            at.pop();
        }
        Type::Ref(_, _, t) | Type::Box(t) => {
            out.push((at.clone(), item.clone()));
            at.push(0);
            out.push((at.clone(), (**t).clone()));
            at.pop();
        }
        t => out.push((at.clone(), t.clone())),
    }
}

/// What type `pattern`, of an impl's parameters, matches in `t` (`pattern`
/// with them replaced): each parameter's part of `t`.
fn bind(pattern: &Type, t: &Type, out: &mut HashMap<String, Type>) -> bool {
    match (pattern, t) {
        (Type::Param(p, _), _) => match out.get(p) {
            Some(u) => u == t,
            None => {
                out.insert(p.clone(), t.clone());
                true
            }
        },
        (Type::Adt(a, _, xs), Type::Adt(b, _, ys)) if a == b && xs.len() == ys.len() => {
            xs.iter().zip(ys).all(|(x, y)| bind(x, y, out))
        }
        (Type::Tuple(xs), Type::Tuple(ys)) if xs.len() == ys.len() => {
            xs.iter().zip(ys).all(|(x, y)| bind(x, y, out))
        }
        (Type::Ref(_, m, x), Type::Ref(_, n, y)) if m == n => bind(x, y, out),
        (a, b) => a == b,
    }
}

/// `tree`, of a value at position `at` of another: of that other value.
pub(super) fn shift(tree: &Tree, at: &[usize]) -> Tree {
    (tree.iter())
        .map(|(p, d)| {
            let mut q = at.to_vec();
            q.extend_from_slice(p);
            (q, d.clone())
        })
        .collect()
}

/// What either of `a` and `b` says: at each position both say something
/// of, one of the two (a position only one says something of may be any
/// value, if the iterator was the other).
fn either(a: &Tree, b: &Tree) -> Tree {
    let mut out = Vec::new();
    for (p, d) in a {
        let theirs: Vec<&Disj> = (b.iter()).filter(|(q, _)| q == p).map(|(_, e)| e).collect();
        if theirs.is_empty() {
            continue;
        }
        let mine = Cand::all(vec![d.clone()]);
        let other = Cand::all(theirs.into_iter().cloned().collect());
        out.push((
            p.clone(),
            Disj(vec![(Vec::new(), mine), (Vec::new(), other)]),
        ));
    }
    out
}

impl Walk<'_> {
    /// Is `symbol`, a function called (before an impl's method is found
    /// for it), a method of the protocol: `Iterator`'s or
    /// `DoubleEndedIterator`'s, taking its iterator by `&mut`? Its name,
    /// if it is.
    pub(super) fn protocol(&self, symbol: &str) -> Option<String> {
        for tr in [ITERATOR, ITERATOR_DEI] {
            let Some(d) = self.cx.traits.traits.get(tr) else {
                continue;
            };
            for m in &d.methods {
                if m.symbol != symbol || !m.receiver {
                    continue;
                }
                let Type::Fn(sig) = &m.scheme.ty else {
                    continue;
                };
                if let Some(Type::Ref(_, true, inner)) = sig.params.first()
                    && matches!(&**inner, Type::Param(p, _) if p == "Self")
                {
                    return Some(m.name.clone());
                }
            }
        }
        None
    }

    /// Is concrete type `t` an adapter: is its `Item` made of the items of
    /// iterators, or the results of closures, it's generic in?
    pub(super) fn adapter(&self, t: &Type) -> bool {
        let Some((i, _)) = self.cx.traits.select(t, Some(ITERATOR), &[]) else {
            return false;
        };
        let d = &self.cx.traits.impls[i];
        let Some((_, item)) = d.assoc.iter().find(|(n, _)| n == "Item") else {
            return false;
        };
        let mut leaves = Vec::new();
        sources(item, &mut Vec::new(), &mut leaves);
        leaves.iter().any(|(_, l)| {
            item_of(l).is_some()
                || matches!(l, Type::Param(u, _) if d.eqs.iter().any(|(a, b)| {
                    matches!((a, b), (Type::Proj(_, _, n), Type::Fn(fs))
                        if n == crate::traits::SIG && matches!(&*fs.ret, Type::Param(v, _) if v == u))
                }))
        })
    }

    /// The items of value `v`, of type `t`: what's kept of its object, or
    /// else what its type's signatures say (`envelope`), which is then
    /// kept of it.
    pub(super) fn items_of(&mut self, v: &Val, t: &Type) -> Option<Rc<Tree>> {
        let o = targs::obj_of(v)?;
        if let Some(g) = self.items.get(&o) {
            return Some(g.clone());
        }
        let g = Rc::new(self.envelope(v, t)?);
        self.items.insert(o, g.clone());
        Some(g)
    }

    /// The items of the value at place `p`, now.
    pub(super) fn items_at(&mut self, p: &Place) -> Option<Rc<Tree>> {
        let t = self.place_ty(p);
        let v = self.read_place(p);
        self.items_of(&v, &t)
    }

    /// Place `p` has a new value, which gives what `g` says, as what it
    /// had did (lemma I1): the value.
    pub(super) fn keep_items(&mut self, p: &Place, g: Rc<Tree>) -> Val {
        let v = self.read_place(p);
        if let Some(o) = targs::obj_of(&v) {
            self.items.insert(o, g);
        }
        v
    }

    /// Of `states`, values of iterator type `t` its protocol got to from
    /// one whose items are `g`: each is within the bounds of what its own
    /// signatures say (`Envelope`), as every state its protocol gets to is
    /// (lemma I2). At a call of its protocol, those are the states before
    /// and after, so of what it gives, its signature says the rest. Of an
    /// iterator that's one of several (`either`), within the bounds of one
    /// of them: so each case of what's said is said apart, and if any case
    /// isn't what `t`'s signatures say, nothing is.
    pub(super) fn bound_states(&mut self, g: &Tree, t: &Type, states: &[Val]) {
        for (pos, d) in g {
            if !pos.is_empty() || d.0.is_empty() {
                continue;
            }
            let mut cases = Vec::new();
            for (guards, c) in &d.0 {
                let Cand::Gives(e) = c else {
                    cases.clear();
                    break;
                };
                if e.ty != *t {
                    cases.clear();
                    break;
                }
                let e = e.clone();
                let mut all = guards.clone();
                for v in states {
                    for (m, at, le, ge) in &e.bounds {
                        let Val::Int(at) = at else {
                            continue;
                        };
                        let recv = Val::Ref(self.new_obj(), Pointee::Val(Box::new(v.clone())));
                        let Val::Int(x) = self.measure(m, recv) else {
                            continue;
                        };
                        if *le {
                            all.extend(Formula::le(&x, at));
                        }
                        if *ge {
                            all.extend(Formula::le(at, &x));
                        }
                    }
                }
                cases.push(Formula::and(all));
            }
            if !cases.is_empty() {
                self.fact(Formula::or(cases));
            }
        }
    }

    /// The impls of `Iterator` and `DoubleEndedIterator` for concrete type
    /// `t`, if it's an iterator whose impls define only `next` and
    /// `next_back`, with their type arguments: the default methods, which
    /// get values only from those, by parametricity, are the rest of its
    /// protocol.
    fn own_impls(&self, t: &Type) -> Option<Vec<OwnMethod>> {
        if !concrete(t) {
            return None;
        }
        let mut out = Vec::new();
        for (tr, name) in [(ITERATOR, "next"), (ITERATOR_DEI, "next_back")] {
            let Some((i, args)) = self.cx.traits.select(t, Some(tr), &[]) else {
                if tr == ITERATOR {
                    return None;
                }
                continue;
            };
            let d = &self.cx.traits.impls[i];
            if d.methods.iter().any(|(n, _)| n != name) {
                return None;
            }
            let (_, symbol) = d.methods.iter().find(|(n, _)| n == name)?;
            let names = d.params.as_slice().to_vec();
            out.push((
                name.to_string(),
                symbol.clone(),
                names.into_iter().zip(args).collect(),
            ));
        }
        Some(out)
    }

    /// Are the protocols of concrete type `t` its impls' `next` and
    /// `next_back` and the default methods (lemma I3)?
    fn defaults_only(&self, t: &Type) -> bool {
        self.own_impls(t).is_some()
    }

    /// What concrete iterator type `t`'s own signatures say of every value
    /// `v`, one of it, gives (lemma I2): nothing, if a method that gives
    /// one says nothing of it.
    fn envelope(&mut self, v: &Val, t: &Type) -> Option<Tree> {
        let own = self.own_impls(t)?;
        let mut tree = self.carried_items(v, t);
        let mut methods = Vec::new();
        for (name, symbol, targs) in own {
            match self.cx.refined.get(&symbol) {
                Some(sig) if sig.ret.is_some() => methods.push((name, sig.clone(), targs)),
                _ => return Some(tree).filter(|t| !t.is_empty()),
            }
        }
        let moves = self.moves(t, &methods);
        let mut bounds = Vec::new();
        for (m, le, ge) in moves {
            if !le && !ge {
                continue;
            }
            let recv = Val::Ref(self.new_obj(), Pointee::Val(Box::new(v.clone())));
            let at = self.measure(&m, recv);
            bounds.push((m, at, le, ge));
        }
        let e = Envelope {
            ty: t.clone(),
            methods,
            bounds,
        };
        tree.push((
            Vec::new(),
            Disj(vec![(Vec::new(), Cand::Gives(Rc::new(e)))]),
        ));
        Some(tree)
    }

    /// What every value iterator `v`, of concrete type `t`, gives has of
    /// what's in it: where its impl's `Item` is one of its type
    /// parameters `X` that's free there, and in each impl of its protocol
    /// (`free_in_protocol`), each value of `X` it gives is one in it
    /// (lemma A2 of docs/60-refined-type-arguments.md), at a position of
    /// its type where nothing else can write it.
    fn carried_items(&mut self, v: &Val, t: &Type) -> Tree {
        let mut tree = Vec::new();
        let Some(impls) = self.protocol_impls(t) else {
            return tree;
        };
        let d = self.cx.traits.impls[impls[0].0].clone();
        let Some((_, item)) = d.assoc.iter().find(|(n, _)| n == "Item") else {
            return tree;
        };
        let params = d.params.as_slice().to_vec();
        if params.len() != impls[0].1.len() {
            return tree;
        }
        let mut leaves = Vec::new();
        sources(item, &mut Vec::new(), &mut leaves);
        for (pos, leaf) in leaves {
            let Type::Param(x, _) = &leaf else {
                continue;
            };
            if !params.contains(x) || !self.free_in_protocol(&impls, &d, x) {
                continue;
            }
            let mut cases = Vec::new();
            for path in targs::param_paths(&d.self_ty, x) {
                if !matches!(self.type_at(t, &path), Some((_, Variance::Co))) {
                    cases.clear();
                    break;
                }
                cases.extend(self.values_at(v, t, &path));
            }
            let d = Disj(cases);
            if !d.0.is_empty() && !d.is_any() {
                tree.push((pos, d));
            }
        }
        tree
    }

    /// The impls of `Iterator` and, if it is one, `DoubleEndedIterator`
    /// for concrete type `t`, with their type arguments: every impl whose
    /// methods give what an iterator of the type gives.
    fn protocol_impls(&self, t: &Type) -> Option<Vec<(usize, Vec<Type>)>> {
        let first = self.cx.traits.select(t, Some(ITERATOR), &[])?;
        let mut out = vec![first];
        if let Some(back) = self.cx.traits.select(t, Some(ITERATOR_DEI), &[]) {
            out.push(back);
        }
        Some(out)
    }

    /// Is the type parameter `x` of impl `d` (the first of `impls`) free
    /// in each of them: is the parameter where `x` is in `d`'s type, in
    /// each, bounded by markers alone and in nothing but the type's
    /// fields? Then none of their methods can make a value of it.
    fn free_in_protocol(
        &self,
        impls: &[(usize, Vec<Type>)],
        d: &crate::traits::ImplDef,
        x: &str,
    ) -> bool {
        let paths = targs::param_paths(&d.self_ty, x);
        if paths.is_empty() {
            return false;
        }
        impls.iter().all(|(i, _)| {
            let dd = &self.cx.traits.impls[*i];
            let Some(y) = same_param_at(&dd.self_ty, &paths) else {
                return false;
            };
            let sig = Signature::of_impl(self.cx, dd);
            sig.occurrences(self.cx, &y).iter().all(|o| match o.site {
                Site::Field(_) => o.shape != Shape::Hidden,
                Site::Pred(_) => {
                    o.of == Of::Itself && o.shape == Shape::Whole && sig.pred_is_marker(o)
                }
                Site::Param(_) | Site::Ret | Site::Eq(_) | Site::SigParam(..) | Site::SigRet(_) => {
                    false
                }
            })
        })
    }

    /// Which way each measure of iterator type `t` may move under each of
    /// `methods`, as their postconditions say, for every value of their
    /// other arguments (as `keeps_at`, which this generalises): a measure
    /// never grows if each says it's at most what it was, and so on. One a
    /// method says nothing of may be anything after it.
    fn moves(&mut self, t: &Type, methods: &[Method]) -> Moves {
        if let Some(m) = self.moves_of.get(t) {
            return m.clone();
        }
        let mut all: Option<Moves> = None;
        for (_, sig, targs) in methods {
            let (before, after) = (self.new_obj(), self.new_obj());
            let behind =
                |w: &mut Self, o: Obj| Val::Ref(w.new_obj(), Pointee::Val(Box::new(Val::Obj(o))));
            let old = vec![behind(self, before)];
            let now = vec![behind(self, after)];
            let start = self.facts.len();
            let reachable = std::mem::replace(&mut self.reachable, true);
            for r in &sig.ensures {
                if let Some(f) = self.pred(r, &Val::Other, Env::Post(&old, Some(&now), targs)) {
                    self.fact(f);
                }
            }
            let asked: Vec<(String, Val)> = (self.measured.iter())
                .filter(|((o, _), _)| *o == after)
                .map(|((_, m), v)| (m.clone(), v.clone()))
                .collect();
            let mut here: Moves = Vec::new();
            for (m, v) in asked {
                if self.cx.measures[&m].unsized_self {
                    continue;
                }
                let recv = behind(self, before);
                let was = self.measure(&m, recv);
                let (Val::Int(now), Val::Int(was)) = (&v, &was) else {
                    continue;
                };
                let holds = |w: &mut Self, f: Option<Formula>| {
                    f.is_some_and(|f| w.proves(Some(&f)).is_ok())
                };
                let le = holds(self, Formula::le(now, was));
                let ge = holds(self, Formula::le(was, now));
                here.push((m, le, ge));
            }
            self.facts.truncate(start);
            self.reachable = reachable;
            all = Some(match all {
                None => here,
                Some(prev) => (prev.into_iter())
                    .filter_map(|(m, le, ge)| {
                        let (_, le2, ge2) = here.iter().find(|(n, ..)| *n == m)?;
                        Some((m, le && *le2, ge && *ge2))
                    })
                    .collect(),
            });
        }
        let mut out = all.unwrap_or_default();
        out.sort();
        self.moves_of.insert(t.clone(), out.clone());
        out
    }

    /// That `v` is a value envelope `e` says its iterator gives (lemma
    /// I2): one some method of it gave, from a state within its bounds,
    /// to one within them, as its refinements say. The states are new
    /// objects each time it's said, as each value is given from a state of
    /// its own.
    pub(super) fn envelope_formula(&mut self, e: &Envelope, v: &Val) -> Option<Formula> {
        let item = self.cx.solved(&Type::Proj(
            Box::new(e.ty.clone()),
            ITERATOR.to_string(),
            "Item".to_string(),
        ));
        let opt = Type::Adt(OPTION.to_string(), Vec::new(), vec![item]);
        let some = self.variant_index(OPTION, "Some")?;
        let mut cases = Vec::new();
        for (_, sig, targs) in &e.methods {
            let (before, after) = (self.new_obj(), self.new_obj());
            let behind =
                |w: &mut Self, o: Obj| Val::Ref(w.new_obj(), Pointee::Val(Box::new(Val::Obj(o))));
            let old = vec![behind(self, before)];
            let now = vec![behind(self, after)];
            let mut all = Vec::new();
            for (m, at, le, ge) in &e.bounds {
                let Val::Int(at) = at else {
                    continue;
                };
                for o in [before, after] {
                    let recv = behind(self, o);
                    let Val::Int(x) = self.measure(m, recv) else {
                        continue;
                    };
                    if *le {
                        all.extend(Formula::le(&x, at));
                    }
                    if *ge {
                        all.extend(Formula::le(at, &x));
                    }
                }
            }
            for r in &sig.ensures {
                if let Some(f) = self.pred(r, &Val::Other, Env::Post(&old, Some(&now), targs)) {
                    all.push(f);
                }
            }
            let o = self.new_obj();
            self.enum_parts(o, &opt, Some((some, vec![v.clone()])));
            let r = sig.ret.as_ref()?;
            all.push(self.pred(r, &Val::Obj(o), Env::Post(&old, Some(&now), targs))?);
            cases.push(Formula::and(all));
        }
        Some(Formula::or(cases))
    }

    /// Of a call of `symbol` (resolved), at type arguments `targs`, with
    /// arguments `vals`, giving `result` of type `ty`: if it's generic in
    /// an iterator, and gives it, or an adapter of it, the items `result`
    /// has (lemmas I3, I4).
    pub(super) fn make_items(
        &mut self,
        symbol: &str,
        targs: &[Type],
        vals: &[Val],
        result: &Val,
        ty: &Type,
    ) {
        let Some(o) = targs::obj_of(result) else {
            return;
        };
        if self.items.contains_key(&o) || !concrete(ty) {
            return;
        }
        let s = self.cx.scheme(symbol).clone();
        let Type::Fn(sig) = &s.ty else {
            return;
        };
        let names = s.params.as_slice().to_vec();
        if names.len() != targs.len() {
            return;
        }
        let g = match &*sig.ret {
            Type::Param(x, _) => self.given_items(&s, x, targs, vals),
            ret => self.adapter_items(&s, ret, targs, vals, ty),
        };
        if let Some(g) = g.filter(|g| !g.is_empty()) {
            self.items.insert(o, Rc::new(g));
        }
    }

    /// Of a call of `symbol` (resolved, at type arguments `targs`, with
    /// arguments `vals`) lending places `places`: each place lent to a
    /// `&mut X`, for an iterator `X` the callee can't make, with what it
    /// has after, one of the iterators it was given or what that one
    /// became by its protocol (lemma I3).
    pub(super) fn lent_items(
        &mut self,
        symbol: &str,
        targs: &[Type],
        vals: &[Val],
        places: &[Option<Place>],
    ) -> Vec<(Place, Rc<Tree>)> {
        let mut out = Vec::new();
        let s = self.cx.scheme(symbol).clone();
        let Type::Fn(sig) = &s.ty else {
            return out;
        };
        if s.params.as_slice().len() != targs.len() {
            return out;
        }
        for (j, p) in places.iter().enumerate() {
            let Some(p) = p else {
                continue;
            };
            let Some(Type::Ref(_, true, inner)) = sig.params.get(j) else {
                continue;
            };
            let Type::Param(x, _) = &**inner else {
                continue;
            };
            let x = x.clone();
            if let Some(g) = self
                .given_items(&s, &x, targs, vals)
                .filter(|g| !g.is_empty())
            {
                out.push((p.clone(), Rc::new(g)));
            }
        }
        out
    }

    /// Is `x`, a type parameter of a function of scheme `s`, at type
    /// arguments `targs`, an iterator it can't make (lemma I3): bounded by
    /// `Iterator` or `DoubleEndedIterator`, one it can only pass on
    /// (`Signature::conserved`), and its impls the defaults? What it is
    /// there, if it is.
    fn conserved(&self, s: &crate::types::Scheme, x: &str, targs: &[Type]) -> Option<Type> {
        let names = s.params.as_slice().to_vec();
        if names.len() != targs.len() {
            return None;
        }
        let i = names.iter().position(|n| n == x)?;
        let at = targs.get(i)?.clone();
        let iterator = (s.preds.iter()).any(|p| {
            matches!(&p.ty, Type::Param(q, _) if q == x)
                && (p.trait_name == ITERATOR || p.trait_name == ITERATOR_DEI)
        });
        let conserved = Signature::of_scheme(s).conserved(self.cx, x);
        (iterator && conserved && self.defaults_only(&at)).then_some(at)
    }

    /// Does a call of `symbol` (resolved, at type arguments `targs`) keep
    /// what the iterator it's lent `&mut` as argument `i` gives: is it
    /// generic in it, an iterator it can't make, and given no other of its
    /// type (lemma I3)? Then what it leaves there is that one, or what
    /// that one became by its protocol.
    pub(super) fn generic_keeps(&self, symbol: &str, targs: &[Type], i: usize) -> bool {
        let s = self.cx.scheme(symbol);
        let Type::Fn(sig) = &s.ty else {
            return false;
        };
        let Some(Type::Ref(_, true, inner)) = sig.params.get(i) else {
            return false;
        };
        let Type::Param(x, _) = &**inner else {
            return false;
        };
        let alone = (sig.params.iter().enumerate()).all(|(j, p)| j == i || !mentions_param(p, x));
        alone && self.conserved(s, x, targs).is_some()
    }

    /// The items of every value of callee type parameter `x` the callee
    /// can give back, if `x` is an iterator it can't make (lemma I3): one
    /// of those it was given, by value or behind a reference, or what
    /// that one became by its protocol.
    pub(super) fn given_items(
        &mut self,
        s: &crate::types::Scheme,
        x: &str,
        targs: &[Type],
        vals: &[Val],
    ) -> Option<Tree> {
        let Type::Fn(sig) = &s.ty else {
            return None;
        };
        let at = self.conserved(s, x, targs)?;
        let mut g: Option<Tree> = None;
        let mut any = false;
        for (j, pt) in sig.params.iter().enumerate() {
            if !mentions_param(pt, x) {
                continue;
            }
            let v = match pt {
                Type::Param(q, _) if q == x => vals.get(j)?.clone(),
                Type::Ref(_, _, inner) if matches!(&**inner, Type::Param(q, _) if q == x) => {
                    let r = vals.get(j)?.clone();
                    self.pointee(r)
                }
                _ => return None,
            };
            any = true;
            let here = (*self.items_of(&v, &at)?).clone();
            g = Some(match g {
                None => here,
                Some(prev) => either(&prev, &here),
            });
        }
        g.filter(|_| any)
    }

    /// The items of `result`, of concrete type `ty`, which the callee,
    /// whose scheme is `s`, gives as its `ret`: if `ty` is an *adapter*,
    /// whose impls of the protocol are generic in iterators and closures
    /// it gets its items from alone (lemmas I4 of docs/82, I4′ of
    /// docs/83), what those it was given give.
    fn adapter_items(
        &mut self,
        s: &crate::types::Scheme,
        ret: &Type,
        targs: &[Type],
        vals: &[Val],
        ty: &Type,
    ) -> Option<Tree> {
        let impls = self.protocol_impls(ty)?;
        let roles = self.adapter_roles(&impls)?;
        let d = self.cx.traits.impls[impls[0].0].clone();
        // What each of its parameters is, as the callee's.
        let mut at = HashMap::new();
        if !bind(&d.self_ty, ret, &mut at) {
            return None;
        }
        let callee = Signature::of_scheme(s);
        let summaries: Vec<(usize, Rc<summary::Summary>)> = self
            .call_summaries
            .iter()
            .map(|(j, s)| (*j, s.clone()))
            .collect();
        let mut tree = Vec::new();
        for (pos, role) in roles {
            match role {
                Role::Items(p) => {
                    let Some(Type::Param(x, _)) = at.get(&p) else {
                        continue;
                    };
                    let x = x.clone();
                    if let Some(g) = self.given_items(s, &x, targs, vals) {
                        tree.extend(shift(&g, &pos));
                    }
                }
                Role::Output(f) => {
                    // The closure is the one the callee was given for it,
                    // its only source of them (lemma V5 of
                    // docs/78-callable-values.md).
                    let Some(Type::Param(g, _)) = at.get(&f) else {
                        continue;
                    };
                    let Some(j) = callee.sole_source(self.cx, g) else {
                        continue;
                    };
                    let summary = match summaries.iter().find(|(k, _)| *k == j) {
                        Some((_, s)) => s.clone(),
                        None => match vals.get(j).and_then(|v| self.callable(v)) {
                            Some(summary::Callable::Closure(s)) => s,
                            _ => continue,
                        },
                    };
                    let cases: Vec<(Vec<Formula>, Cand)> = (0..summary.rets.len())
                        .map(|k| {
                            let made = summary::Made {
                                summary: summary.clone(),
                                ret: k,
                                pos: Vec::new(),
                            };
                            (Vec::new(), Cand::Made(Rc::new(made)))
                        })
                        .collect();
                    tree.push((pos, Disj(cases)));
                }
            }
        }
        Some(tree)
    }

    /// Of an adapter whose impls of the protocol are `impls` (`Iterator`'s
    /// first): where each value it gives comes from, by position in its
    /// `Item`, as its `Iterator` impl's parameters (lemma I4′ of docs/83).
    /// A position whose values come from an iterator of its own, whose
    /// items come from its protocol alone (`items_from_protocol`), or from
    /// a closure it holds, the one source of its result type
    /// (`closure_output`, `held`), in every impl of the protocol: the
    /// parameter at the same place in each impl's type has the same role
    /// there. `None` if the protocol's methods could get items from a
    /// closure (`protocol_methods_harmless`).
    fn adapter_roles(&self, impls: &[(usize, Vec<Type>)]) -> Option<Vec<(Pos, Role)>> {
        if !sources::protocol_methods_harmless(self.cx) {
            return None;
        }
        let d = &self.cx.traits.impls[impls[0].0];
        let item = d.assoc.iter().find(|(n, _)| n == "Item")?.1.clone();
        let mut leaves = Vec::new();
        sources(&item, &mut Vec::new(), &mut leaves);
        let sig = Signature::of_impl(self.cx, d);
        let mut out = Vec::new();
        for (pos, leaf) in leaves {
            // Of an iterator: the role is its place in the type.
            if let Some(p) = item_of(&leaf) {
                let paths = targs::param_paths(&d.self_ty, p);
                let good = !paths.is_empty()
                    && impls.iter().all(|(i, _)| {
                        let dd = &self.cx.traits.impls[*i];
                        let Some(q) = same_param_at(&dd.self_ty, &paths) else {
                            return false;
                        };
                        let iterator = dd.preds.iter().any(|pr| {
                            matches!(&pr.ty, Type::Param(r, _) if *r == q)
                                && (pr.trait_name == ITERATOR || pr.trait_name == ITERATOR_DEI)
                        });
                        iterator && Signature::of_impl(self.cx, dd).items_from_protocol(self.cx, &q)
                    });
                if !good {
                    return None;
                }
                out.push((pos, Role::Items(p.to_string())));
                continue;
            }
            // Of a closure's result: the role is the closure's place.
            let Type::Param(u, _) = &leaf else {
                continue;
            };
            let Some(f) = sig.closure_output(self.cx, u) else {
                continue;
            };
            let paths = targs::param_paths(&d.self_ty, &f);
            let good = !paths.is_empty()
                && impls.iter().all(|(i, _)| {
                    let dd = &self.cx.traits.impls[*i];
                    let Some(g) = same_param_at(&dd.self_ty, &paths) else {
                        return false;
                    };
                    let dsig = Signature::of_impl(self.cx, dd);
                    let output = match dsig.sig_of(&g).map(|fs| &*fs.ret) {
                        Some(Type::Param(v, _)) => dsig.closure_output(self.cx, v),
                        _ => None,
                    };
                    output.as_deref() == Some(g.as_str()) && dsig.held(self.cx, &g)
                });
            if !good {
                return None;
            }
            out.push((pos, Role::Output(f)));
        }
        Some(out)
    }

    /// What each closure given straight to a call of `symbol` (resolved,
    /// at type arguments `targs`, with arguments `vals`), at argument
    /// index in `idx`, is given, by parameter, of the items of the
    /// iterators it's given with: a closure the callee may call with a
    /// value of `X::Item`, for an iterator `X` it can't make whose items
    /// come from its protocol alone, is called only with what the
    /// iterators it was given give (lemmas I3, I4 of docs/82, I3′ of
    /// docs/83); but only one the callee can't keep, to be called later
    /// with anything, or keeps in a sealed adapter (`sealed_in_result`).
    pub(super) fn item_params(
        &mut self,
        symbol: &str,
        targs: &[Type],
        vals: &[Val],
        idx: &[usize],
    ) -> HashMap<usize, Vec<Tree>> {
        let mut out = HashMap::new();
        let s = self.cx.scheme(symbol).clone();
        let Type::Fn(sig) = &s.ty else {
            return out;
        };
        if s.params.as_slice().len() != targs.len() {
            return out;
        }
        let callee = Signature::of_scheme(&s);
        for &j in idx {
            let Some(Type::Param(f, _)) = sig.params.get(j) else {
                continue;
            };
            let Some(fs) = callee.sig_of(f).cloned() else {
                continue;
            };
            let mut trees = Vec::new();
            for pt in &fs.params {
                // The item itself, or what a reference to it points to.
                let (x, at) = match pt {
                    t if item_of(t).is_some() => (item_of(t), Vec::new()),
                    Type::Ref(_, _, inner) => (item_of(inner), vec![0]),
                    _ => (None, Vec::new()),
                };
                let g = match x {
                    Some(x) if callee.items_from_protocol(self.cx, x) => {
                        let x = x.to_string();
                        self.given_items(&s, &x, targs, vals)
                    }
                    _ => None,
                };
                trees.push(g.map(|g| shift(&g, &at)).unwrap_or_default());
            }
            if trees.iter().all(|t| t.is_empty()) {
                continue;
            }
            if callee.kept(self.cx, f, j) && !self.sealed_in_result(&s, &callee, f, j, targs, &fs) {
                continue;
            }
            out.insert(j, trees);
        }
        out
    }

    /// Does the callee, of scheme `s` (`callee`), keep the closure it's
    /// given as parameter `j`, of type parameter `f` (signature `fs`),
    /// only in a *sealed* adapter it gives back (lemma S of docs/83)? Its
    /// result is an adapter `S<.., F, ..>`, `f` only there and nowhere else
    /// it could be kept; `S` is sealed (`sealed`); and in each impl of
    /// `S`'s protocol, the closure there is called with what an iterator
    /// there gives, items from its protocol alone, where the callee's
    /// closure is with an `X::Item`, at the place in `S`'s type where the
    /// callee puts its `X`.
    fn sealed_in_result(
        &self,
        s: &crate::types::Scheme,
        callee: &Signature,
        f: &str,
        j: usize,
        targs: &[Type],
        fs: &crate::types::FnType,
    ) -> bool {
        if callee.sole_source(self.cx, f) != Some(j) {
            return false;
        }
        let Some(ret) = callee.ret() else {
            return false;
        };
        let Type::Adt(symbol, _, args) = ret else {
            return false;
        };
        // In the result only as one of the adapter's type arguments.
        let only_there = args
            .iter()
            .all(|a| matches!(a, Type::Param(g, _) if g == f) || !free::mentions(a, f));
        let elsewhere = callee.occurrences(self.cx, f).iter().any(|o| match o.site {
            Site::Pred(k) => {
                !(o.of == Of::Itself && o.shape == Shape::Whole && callee.pred_is_fn_or_marker(k))
            }
            Site::Param(i) => i != j,
            Site::SigParam(k, _) => !callee.sig_is(k, f),
            Site::Ret => false,
            Site::Field(_) | Site::Eq(_) | Site::SigRet(_) => true,
        });
        if !only_there || elsewhere || sources::has_opaque(ret) || !self.sealed(symbol) {
            return false;
        }
        // The adapter's type, as the callee gives it at this call.
        let names = s.params.as_slice().to_vec();
        if names.len() != targs.len() {
            return false;
        }
        let subst = crate::types::Subst::new(&names, targs);
        let ty = self.cx.solved(&subst.apply(ret));
        let Some(impls) = self.protocol_impls(&ty) else {
            return false;
        };
        let paths = targs::param_paths(ret, f);
        impls.iter().all(|(i, _)| {
            let dd = &self.cx.traits.impls[*i];
            let Some(g) = same_param_at(&dd.self_ty, &paths) else {
                return false;
            };
            let dsig = Signature::of_impl(self.cx, dd);
            let Some(gs) = dsig.sig_of(&g) else {
                return false;
            };
            if !dsig.held(self.cx, &g) || gs.params.len() != fs.params.len() {
                return false;
            }
            // Each parameter the callee's closure is given an item at, the
            // adapter's is too, of an iterator at the same place.
            let mut at = HashMap::new();
            if !bind(&dd.self_ty, ret, &mut at) {
                return false;
            }
            fs.params.iter().zip(&gs.params).all(|(mine, theirs)| {
                let item = |t: &Type| match t {
                    t if item_of(t).is_some() => item_of(t).map(|x| (x.to_string(), false)),
                    Type::Ref(_, m, inner) => item_of(inner).map(|x| (x.to_string(), *m)),
                    _ => None,
                };
                let Some((x, m)) = item(mine) else {
                    return true;
                };
                let Some((p, n)) = item(theirs) else {
                    return false;
                };
                m == n
                    && matches!(at.get(&p), Some(Type::Param(y, _)) if *y == x)
                    && dsig.items_from_protocol(self.cx, &p)
            })
        })
    }

    /// Is struct or enum `symbol` *sealed* where the walk is (lemma S of
    /// docs/83): its fields are seen in its module alone, which the code
    /// walked isn't in; and no function of that module (or one inside it)
    /// takes a value of it, or of a type that holds one, but the methods of
    /// its impls of the protocol, and impls derived for markers and
    /// `Discard`? Then what its values hold is reached only through those.
    fn sealed(&self, symbol: &str) -> bool {
        let cx = self.cx;
        let Some(def) = cx.types.iter().find(|d| d.symbol == symbol) else {
            return false;
        };
        if cx.modules.within(cx.module, def.module) {
            return false;
        }
        if def
            .fields
            .iter()
            .any(|(_, level)| *level != crate::ast::Level::Module)
        {
            return false;
        }
        // It, and every type whose fields hold one of these.
        let mut holders = vec![symbol.to_string()];
        loop {
            let more: Vec<String> = (cx.types.iter())
                .filter(|t| !holders.contains(&t.symbol))
                .filter(|t| {
                    declared_fields(cx, &t.symbol)
                        .iter()
                        .any(|f| holds(f, &holders))
                })
                .map(|t| t.symbol.clone())
                .collect();
            if more.is_empty() {
                break;
            }
            holders.extend(more);
        }
        // The methods that may take one.
        let mut allowed: Vec<&str> = Vec::new();
        for d in &cx.traits.impls {
            let Type::Adt(of, ..) = &d.self_ty else {
                continue;
            };
            let protocol =
                of == symbol && matches!(d.trait_name.as_deref(), Some(ITERATOR | ITERATOR_DEI));
            let derived = d.derived
                && holders.contains(of)
                && d.trait_name.as_deref().is_some_and(sources::marker);
            if protocol || derived {
                allowed.extend(d.methods.iter().map(|(_, m)| m.as_str()));
            }
        }
        cx.fns.iter().all(|f| {
            !cx.modules.within(f.module, def.module)
                || allowed.contains(&f.symbol.as_str())
                || !takes(&f.scheme, &holders)
        })
    }
}

/// Where each value an adapter gives comes from (`Walk::adapter_roles`).
enum Role {
    /// The items of its iterator parameter `.0`.
    Items(String),
    /// What its closure parameter `.0` gives.
    Output(String),
}

/// The type parameter at each of `paths` in `t`, if it's one and the
/// same.
fn same_param_at(t: &Type, paths: &[Pos]) -> Option<String> {
    let mut out: Option<String> = None;
    for p in paths {
        let Some(Type::Param(q, _)) = at_path(t, p) else {
            return None;
        };
        match &out {
            Some(o) if o != q => return None,
            _ => out = Some(q.clone()),
        }
    }
    out
}

/// The type at position `p` of `t`, as `targs::param_paths` numbers them.
fn at_path<'t>(t: &'t Type, p: &[usize]) -> Option<&'t Type> {
    let Some((&k, rest)) = p.split_first() else {
        return Some(t);
    };
    let child = match t {
        Type::Adt(_, _, args) => args.get(k)?,
        Type::Box(i) | Type::Ref(_, _, i) | Type::Slice(i) | Type::Array(i, _) if k == 0 => i,
        Type::Tuple(ts) => ts.get(k)?,
        _ => return None,
    };
    at_path(child, rest)
}

/// The types of struct or enum `symbol`'s fields, as declared.
fn declared_fields(cx: &Checker, symbol: &str) -> Vec<Type> {
    let Some(def) = cx.adts.get(symbol) else {
        return Vec::new();
    };
    let params: Vec<Type> = (def.params.as_slice().iter())
        .map(|p| def.params.ty(p))
        .collect();
    let lifetimes = (def.lifetimes.iter()).map(|l| crate::types::Region::Named(l.clone()));
    let t = Type::Adt(symbol.to_string(), lifetimes.collect(), params);
    match &def.kind {
        crate::types::AdtKind::Struct(_) => (struct_fields(&cx.adts, &t))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, f)| f)
            .collect(),
        crate::types::AdtKind::Enum(vs) => (0..vs.len())
            .flat_map(|v| crate::types::variant_fields(&cx.adts, &t, v))
            .collect(),
    }
}

/// Does `t` mention one of the structs or enums `symbols`?
fn holds(t: &Type, symbols: &[String]) -> bool {
    matches!(t, Type::Adt(s, ..) if symbols.contains(s)) || t.parts().any(|p| holds(p, symbols))
}

/// Does a function of scheme `s` take a value of one of `symbols`, or get
/// one any way but making it: is one in its signature anywhere but its
/// result?
fn takes(s: &crate::types::Scheme, symbols: &[String]) -> bool {
    let Type::Fn(sig) = &s.ty else {
        return true;
    };
    sig.params.iter().any(|p| holds(p, symbols))
        || s.preds.iter().any(|p| holds(&p.ty, symbols))
        || s.eqs
            .iter()
            .any(|(a, b)| holds(a, symbols) || holds(b, symbols))
}

/// `core::option::Option`'s symbol.
const OPTION: &str = "core::option::Option";
