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

use super::*;
use crate::traits::{COPY, DISCARD, SEND, SYNC};
use std::rc::Rc;
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
    /// `next`, and `next_back` if it's double-ended: what they say, and
    /// their impls' type arguments.
    methods: Vec<(RefinedSig, Vec<(String, Type)>)>,
    /// Each measure, its value at the object, and whether it never grows
    /// and never shrinks.
    bounds: Vec<(String, Val, bool, bool)>,
}

/// Which way each measure a protocol call keeps may move: whether it
/// never grows, and whether it never shrinks.
type Moves = Vec<(String, bool, bool)>;

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
        Type::Proj(base, tr, name)
            if name == "Item" && (tr == ITERATOR || tr == ITERATOR_DEI) =>
        {
            match &**base {
                Type::Param(p, _) => Some(p),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Does `t` mention `P::Item` for a `P` in `ps`, or type parameter `u` in
/// `us`?
fn mentions(t: &Type, ps: &[String], us: &[String]) -> bool {
    if item_of(t).is_some_and(|p| ps.iter().any(|q| q == p)) {
        return true;
    }
    if let Type::Param(u, _) = t
        && us.contains(u)
    {
        return true;
    }
    t.parts().any(|s| mentions(s, ps, us))
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
        out.push((p.clone(), Disj(vec![(Vec::new(), mine), (Vec::new(), other)])));
    }
    out
}

impl Walk<'_> {
    /// Is `symbol`, a function called (before an impl's method is found
    /// for it), a method of the protocol: `Iterator`'s or
    /// `DoubleEndedIterator`'s, taking its iterator by `&mut`? And does it
    /// give a value, as `next` and `next_back` do?
    pub(super) fn protocol(&self, symbol: &str) -> Option<bool> {
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
                    return Some(m.name == "next" || m.name == "next_back");
                }
            }
        }
        None
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
    /// had did (lemma I1).
    pub(super) fn keep_items(&mut self, p: &Place, g: Rc<Tree>) {
        let v = self.read_place(p);
        if let Some(o) = targs::obj_of(&v) {
            self.items.insert(o, g);
        }
    }

    /// The impls of `Iterator` and `DoubleEndedIterator` for concrete type
    /// `t`, if it's an iterator whose impls define only `next` and
    /// `next_back`, with their type arguments: the default methods, which
    /// get values only from those, by parametricity, are the rest of its
    /// protocol.
    fn own_impls(&self, t: &Type) -> Option<Vec<(String, Vec<(String, Type)>)>> {
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
            out.push((symbol.clone(), names.into_iter().zip(args).collect()));
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
        let mut methods = Vec::new();
        for (symbol, targs) in own {
            let sig = self.cx.refined.get(&symbol)?.clone();
            sig.ret.as_ref()?;
            methods.push((sig, targs));
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
        Some(vec![(Vec::new(), Disj(vec![(Vec::new(), Cand::Gives(Rc::new(e)))]))])
    }

    /// Which way each measure of iterator type `t` may move under each of
    /// `methods`, as their postconditions say, for every value of their
    /// other arguments (as `keeps_at`, which this generalises): a measure
    /// never grows if each says it's at most what it was, and so on. One a
    /// method says nothing of may be anything after it.
    fn moves(&mut self, t: &Type, methods: &[(RefinedSig, Vec<(String, Type)>)]) -> Moves {
        if let Some(m) = self.moves_of.get(t) {
            return m.clone();
        }
        let mut all: Option<Moves> = None;
        for (sig, targs) in methods {
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
        for (sig, targs) in &e.methods {
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
            if let Some(g) = self.given_items(&s, &x, targs, vals).filter(|g| !g.is_empty()) {
                out.push((p.clone(), Rc::new(g)));
            }
        }
        out
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
        let names = s.params.as_slice().to_vec();
        let i = names.iter().position(|n| n == x)?;
        let at = targs.get(i)?.clone();
        // Bounds that let it make one: any but the protocol's traits and
        // markers, or a closure that gives one.
        let mut iterator = false;
        for p in &s.preds {
            if !mentions_param(&p.ty, x) {
                continue;
            }
            match &p.ty {
                Type::Param(q, _) if q == x => {
                    if p.trait_name == ITERATOR || p.trait_name == ITERATOR_DEI {
                        iterator = true;
                    } else if ![COPY, DISCARD, SEND, SYNC].contains(&p.trait_name.as_str()) {
                        return None;
                    }
                }
                // Of its items, `X::Item: Discard`: they make no `X`.
                _ => {}
            }
        }
        for (a, b) in &s.eqs {
            if let (Type::Proj(_, _, n), Type::Fn(fs)) = (a, b)
                && n == crate::traits::SIG
                && mentions_param(&fs.ret, x)
            {
                return None;
            }
        }
        if !iterator || !self.defaults_only(&at) {
            return None;
        }
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
    /// whose impl of `Iterator` is generic in iterators and closures it
    /// gets its items from alone (lemma I4), what those it was given give.
    fn adapter_items(
        &mut self,
        s: &crate::types::Scheme,
        ret: &Type,
        targs: &[Type],
        vals: &[Val],
        ty: &Type,
    ) -> Option<Tree> {
        let Type::Fn(sig) = &s.ty else {
            return None;
        };
        let (i, _) = self.cx.traits.select(ty, Some(ITERATOR), &[])?;
        let d = self.cx.traits.impls[i].clone();
        let item = d.assoc.iter().find(|(n, _)| n == "Item")?.1.clone();
        let mut leaves = Vec::new();
        sources(&item, &mut Vec::new(), &mut leaves);
        // Its iterators: its parameters bounded by the protocol's traits,
        // and markers, alone; its items' bounds markers alone.
        let params = d.params.as_slice().to_vec();
        let mut iters: Vec<String> = Vec::new();
        for p in &d.preds {
            match &p.ty {
                Type::Param(q, _) if p.trait_name == ITERATOR || p.trait_name == ITERATOR_DEI => {
                    if !iters.contains(q) {
                        iters.push(q.clone());
                    }
                }
                _ => {}
            }
        }
        for p in &d.preds {
            let ok = [COPY, DISCARD, SEND, SYNC].contains(&p.trait_name.as_str());
            match &p.ty {
                Type::Param(q, _) if iters.contains(q) => {
                    if !ok && p.trait_name != ITERATOR && p.trait_name != ITERATOR_DEI {
                        return None;
                    }
                }
                t if mentions(t, &iters, &[]) && !ok => return None,
                _ => {}
            }
        }
        // Its closures: each `F: FnMut(..) -> U` whose `U` is a parameter
        // of its own. No closure gives an item of an iterator, and no
        // equality bound says what one is.
        let mut outs: Vec<(String, String)> = Vec::new();
        for (a, b) in &d.eqs {
            if let (Type::Proj(base, _, n), Type::Fn(fs)) = (a, b)
                && n == crate::traits::SIG
                && let Type::Param(f, _) = &**base
            {
                if mentions(&fs.ret, &iters, &[]) {
                    return None;
                }
                if let Type::Param(u, _) = &*fs.ret
                    && params.contains(u)
                {
                    outs.push((u.clone(), f.clone()));
                }
                continue;
            }
            if mentions(a, &iters, &[]) || mentions(b, &iters, &[]) {
                return None;
            }
        }
        let us: Vec<String> = outs.iter().map(|(u, _)| u.clone()).collect();
        // A closure's result may be a bound's subject only as a marker's.
        for p in &d.preds {
            if mentions(&p.ty, &[], &us) && ![COPY, DISCARD, SEND, SYNC].contains(&p.trait_name.as_str())
            {
                return None;
            }
        }
        // None of them is in its fields, where it could keep one it wasn't
        // given by its iterator or closure.
        let fields = struct_fields(&self.cx.adts, &d.self_ty)?;
        if fields.iter().any(|(_, ft)| mentions(ft, &iters, &us)) {
            return None;
        }
        // What each of its parameters is, as the callee's.
        let mut at = HashMap::new();
        if !bind(&d.self_ty, ret, &mut at) {
            return None;
        }
        let summaries: Vec<(usize, Rc<summary::Summary>)> =
            self.call_summaries.iter().map(|(j, s)| (*j, s.clone())).collect();
        let mut tree = Vec::new();
        for (pos, leaf) in leaves {
            if let Some(p) = item_of(&leaf) {
                if !iters.iter().any(|q| q == p) {
                    continue;
                }
                let Some(Type::Param(x, _)) = at.get(p) else {
                    continue;
                };
                let x = x.clone();
                if let Some(g) = self.given_items(s, &x, targs, vals) {
                    tree.extend(shift(&g, &pos));
                }
                continue;
            }
            let Type::Param(u, _) = &leaf else {
                continue;
            };
            let Some((_, f)) = outs.iter().find(|(v, _)| v == u) else {
                continue;
            };
            // The closure is the one the callee was given for it, its
            // only source of them (lemma V5 of docs/78-callable-values.md).
            let Some(Type::Param(g, _)) = at.get(f) else {
                continue;
            };
            let given: Vec<usize> = (sig.params.iter().enumerate())
                .filter(|(_, pt)| mentions_param(pt, g))
                .map(|(j, _)| j)
                .collect();
            let [j] = given.as_slice() else {
                continue;
            };
            if !matches!(&sig.params[*j], Type::Param(q, _) if q == g) {
                continue;
            }
            let summary = match summaries.iter().find(|(k, _)| k == j) {
                Some((_, s)) => s.clone(),
                None => match vals.get(*j).and_then(|v| self.callable(v)) {
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
        Some(tree)
    }

    /// What each closure given straight to a call of `symbol` (resolved,
    /// at type arguments `targs`, with arguments `vals`), at argument
    /// index in `idx`, is given, by parameter, of the items of the
    /// iterators it's given with: a closure the callee may call with a
    /// value of `X::Item`, for an iterator `X` it can't make, is called
    /// only with what the iterators it was given give (lemmas I3, I4).
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
        for &j in idx {
            let Some(Type::Param(f, _)) = sig.params.get(j) else {
                continue;
            };
            let Some(Type::Fn(fs)) = (s.eqs.iter())
                .find(|(a, _)| {
                    matches!(a, Type::Proj(b, _, n)
                        if n == crate::traits::SIG
                            && matches!(&**b, Type::Param(q, _) if q == f))
                })
                .map(|(_, b)| b)
            else {
                continue;
            };
            let mut trees = Vec::new();
            for pt in &fs.params {
                // The item itself, or what a `&` to it points to.
                let (x, at) = match pt {
                    t if item_of(t).is_some() => (item_of(t), Vec::new()),
                    Type::Ref(_, _, inner) => (item_of(inner), vec![0]),
                    _ => (None, Vec::new()),
                };
                let g = match x {
                    Some(x) => {
                        let x = x.to_string();
                        self.given_items(&s, &x, targs, vals)
                    }
                    None => None,
                };
                trees.push(g.map(|g| shift(&g, &at)).unwrap_or_default());
            }
            if trees.iter().any(|t| !t.is_empty()) {
                out.insert(j, trees);
            }
        }
        out
    }
}

/// `core::option::Option`'s symbol.
const OPTION: &str = "core::option::Option";
