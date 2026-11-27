//! Where generic code gets values of a type parameter
//! (docs/83-sources.md): the one analysis every rule that rests on
//! parametricity asks.
//!
//! Code generic in `Z` can't make a value of `Z` (Reynolds): every one it
//! has came to it some way its signature says. A function's are its
//! parameters, the bounds it can call methods of, its equality bounds, and
//! the closures it's given, through what they give and what they can
//! write back through a parameter; an impl's, its bounds and equalities,
//! its closures, and what its type's fields hold. `Signature::occurrences`
//! finds every place a type parameter is in a signature, each with where
//! it is (`Site`) and how (`Shape`); it reads each part of a scheme or an
//! impl by name, and every kind of type, so a new part or a new kind is a
//! compile error here, not a way in nobody listed (lemma P1). Each rule is
//! then a question about those occurrences that refuses any it doesn't
//! know to be harmless (lemma P2):
//!
//! - `conserved`: an iterator code can't make, only pass on (lemma I3 of
//!   docs/82);
//! - `items_from_protocol`: an iterator whose items come only from its
//!   own protocol, not from a bound that says what they are, nor a value
//!   given (lemma I3′);
//! - `closure_output`: the one closure an adapter's `U` comes from (lemma
//!   I4 of docs/82);
//! - `sole_source`: the one parameter every value of a closure type comes
//!   from (lemma V5 of docs/78);
//! - `kept`: a closure the callee may keep, to be called later with
//!   anything (lemma W7 of docs/63, and `Walk::item_params`);
//! - `makes`: a bound or closure that makes values of a type parameter the
//!   call can't follow (lemma A2 of docs/60).

use super::*;
use crate::borrowck::regions::Variance;
use crate::traits::{COPY, DISCARD, FN_TRAITS, Pred, SEND, SYNC};
use crate::types::FnType;

/// `Iterator` and `DoubleEndedIterator`, whose `Item` is an iterator's
/// items.
const ITERATOR: &str = crate::check::ITERATOR;
const ITERATOR_DEI: &str = "core::iter::DoubleEndedIterator";
const INTO_ITERATOR: &str = "core::iter::IntoIterator";

/// The markers: traits with no methods, whose bounds give nothing.
pub(super) fn marker(name: &str) -> bool {
    [COPY, DISCARD, SEND, SYNC].contains(&name)
}

/// A signature, read for where values of its type parameters can come
/// from: a function's, or an impl's.
#[derive(Clone, Debug)]
pub(super) struct Signature {
    preds: Vec<Pred>,
    /// Its equality bounds, but those that are closures' signatures.
    eqs: Vec<(Type, Type)>,
    /// Each closure type parameter's signature: `F::SIG = fn(..) -> ..`.
    sigs: Vec<(String, FnType)>,
    /// A function's parameters' types; none for an impl.
    params: Vec<Type>,
    /// A function's result; none for an impl.
    ret: Option<Type>,
    /// An impl's type's fields' types (an enum's variants' all); none for
    /// a function. An impl whose type's fields aren't known has `None`,
    /// and nothing's harmless of it.
    fields: Option<Vec<Type>>,
}

/// Where in a signature a type parameter is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Site {
    /// Parameter `.0`'s type: what the function is given.
    Param(usize),
    /// The result's: what it gives back.
    Ret,
    /// Field `.0`'s type, of the impl's type: what its values hold.
    Field(usize),
    /// Bound `.0`'s subject: `T: Tr`, whose methods it can call.
    Pred(usize),
    /// Equality bound `.0`, either side: what it says a type is.
    Eq(usize),
    /// Closure signature `.0`'s parameter `.1`: what the closure's given,
    /// or, where it can put a value, gives back.
    SigParam(usize, usize),
    /// Closure signature `.0`'s result: what the closure gives.
    SigRet(usize),
}

/// What's there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Of {
    /// The type parameter itself.
    Itself,
    /// Its items, `Z::Item` of `Iterator` or `DoubleEndedIterator`.
    Items,
    /// Another projection of it, or one of a type it's in.
    Other,
}

/// How it's there, from the type at the site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    /// The whole type: `Z`.
    Whole,
    /// Behind a shared reference: `&Z`.
    Shared,
    /// Further in, where it's covariant, as in `Option<Z>`, `(Z, u8)` or
    /// `&[Z]`: what has one there can read it, not put another there.
    Read,
    /// Where another can be put: behind a `&mut`, in a `Cell` (a position
    /// that isn't covariant).
    Write,
    /// Where nothing's followed: in a function's type, an `impl` type, a
    /// projection's type, or deeper than looked.
    Hidden,
}

/// One place a type parameter is in a signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Occ {
    pub(super) site: Site,
    pub(super) of: Of,
    pub(super) shape: Shape,
}

impl Occ {
    /// Is it there only to be read: whole, behind a `&`, or covariant
    /// further in?
    fn given(&self) -> bool {
        matches!(self.shape, Shape::Whole | Shape::Shared | Shape::Read)
    }
}

/// How deep a type is looked into.
const DEPTH: usize = 16;

/// Is `t` `<P as Iterator>::Item` (or `DoubleEndedIterator`'s), of type
/// parameter `P`: which `P`?
pub(super) fn items_of(t: &Type) -> Option<&str> {
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

/// Every place type parameter `z` is in `t`, found at `site`, into `out`.
fn find(cx: &Checker, t: &Type, z: &str, site: Site, out: &mut Vec<Occ>) {
    #[allow(clippy::too_many_arguments)]
    fn go(
        cx: &Checker,
        t: &Type,
        z: &str,
        site: Site,
        depth: usize,
        shared: bool,
        v: Variance,
        hidden: bool,
        out: &mut Vec<Occ>,
    ) {
        let leaf = |of: Of| {
            let shape = match () {
                _ if hidden => Shape::Hidden,
                _ if depth == 0 => Shape::Whole,
                _ if depth == 1 && shared => Shape::Shared,
                _ if v == Variance::Co => Shape::Read,
                _ => Shape::Write,
            };
            Occ { site, of, shape }
        };
        if !free::mentions(t, z) {
            return;
        }
        if depth > DEPTH {
            out.push(Occ {
                site,
                of: Of::Other,
                shape: Shape::Hidden,
            });
            return;
        }
        let inner = |child: &Type, cv: Variance, h: bool, sh: bool, out: &mut Vec<Occ>| {
            go(
                cx,
                child,
                z,
                site,
                depth + 1,
                sh,
                v.then(cv),
                hidden || h,
                out,
            )
        };
        match t {
            Type::Param(p, _) if p == z => out.push(leaf(Of::Itself)),
            Type::Proj(base, ..) if matches!(&**base, Type::Param(p, _) if p == z) => {
                let of = if items_of(t).is_some() {
                    Of::Items
                } else {
                    Of::Other
                };
                out.push(leaf(of));
            }
            // A projection of a type `z` is in: what it is depends on an
            // impl, which nothing here follows.
            Type::Proj(..) => out.push(Occ {
                site,
                of: Of::Other,
                shape: Shape::Hidden,
            }),
            Type::Ref(_, m, u) => {
                let cv = if *m { Variance::Inv } else { Variance::Co };
                inner(u, cv, false, !*m && depth == 0, out)
            }
            Type::Box(u) | Type::Slice(u) | Type::Array(u, _) => {
                inner(u, Variance::Co, false, false, out)
            }
            Type::Tuple(ts) => {
                for u in ts {
                    inner(u, Variance::Co, false, false, out);
                }
            }
            Type::Adt(symbol, lts, args) => {
                let info = cx.adt_info();
                let vs = info.variances(symbol).map(<[Variance]>::to_vec);
                for (k, u) in args.iter().enumerate() {
                    let cv = (vs.as_ref())
                        .and_then(|vs| vs.get(lts.len() + k).copied())
                        .unwrap_or(Variance::Inv);
                    inner(u, cv, false, false, out);
                }
            }
            // What's in a function's type, an `impl` type or a type not
            // worked out yet isn't followed.
            Type::Fn(_) | Type::Opaque(..) | Type::Inferred(..) | Type::Var(_) => {
                for u in t.parts() {
                    inner(u, Variance::Inv, true, false, out);
                }
            }
            // Sets of effects are no values.
            Type::Effects(_) => {}
            Type::Param(..)
            | Type::Int(_)
            | Type::Float(_)
            | Type::Nat(_)
            | Type::Const(_)
            | Type::Bool
            | Type::Char
            | Type::Str
            | Type::Unit
            | Type::Never
            | Type::Alloc
            | Type::Panic
            | Type::Global
            | Type::Unsafe => {}
        }
    }
    go(cx, t, z, site, 0, false, Variance::Co, false, out)
}

impl Signature {
    /// A function's signature: its scheme's. Every part of a scheme is
    /// read here, by name.
    pub(super) fn of_scheme(s: &crate::types::Scheme) -> Signature {
        let crate::types::Scheme {
            // Names, and lifetimes and what outlives what: no values.
            params: _,
            lifetimes: _,
            bounds: _,
            preds,
            eqs,
            // Sets of effects: no values.
            effect_bounds: _,
            ty,
        } = s;
        let (params, ret) = match ty {
            Type::Fn(sig) => (sig.params.clone(), (*sig.ret).clone()),
            t => (Vec::new(), t.clone()),
        };
        let mut out = Signature {
            preds: preds.clone(),
            eqs: Vec::new(),
            sigs: Vec::new(),
            params,
            ret: Some(ret),
            fields: None,
        };
        out.split_eqs(eqs);
        out
    }

    /// An impl's signature: its bounds and what its type's fields hold.
    /// Every part of an impl is read here, by name.
    pub(super) fn of_impl(cx: &Checker, d: &crate::traits::ImplDef) -> Signature {
        let crate::traits::ImplDef {
            span: _,
            trait_name: _,
            // Names, and lifetimes and what outlives what: no values.
            params: _,
            lifetimes: _,
            bounds: _,
            self_ty,
            preds,
            eqs,
            // Sets of effects: no values.
            effect_bounds: _,
            // What its associated types are, which is what it gives, not
            // where it gets it; and which atoms of its associated effects
            // hang on bounds: no values.
            assoc: _,
            assoc_unless: _,
            // Checked with its bounds, which are these, and their own,
            // which are the trait's (`protocol_methods_harmless`).
            methods: _,
            derived: _,
        } = d;
        let fields = match self_ty {
            Type::Adt(symbol, ..) => match cx.adts.get(symbol).map(|a| &a.kind) {
                Some(crate::types::AdtKind::Struct(_)) => struct_fields(&cx.adts, self_ty)
                    .map(|fs| fs.into_iter().map(|(_, t)| t).collect()),
                Some(crate::types::AdtKind::Enum(vs)) => Some(
                    (0..vs.len())
                        .flat_map(|v| crate::types::variant_fields(&cx.adts, self_ty, v))
                        .collect(),
                ),
                None => None,
            },
            _ => None,
        };
        let mut out = Signature {
            preds: preds.clone(),
            eqs: Vec::new(),
            sigs: Vec::new(),
            params: Vec::new(),
            ret: None,
            fields,
        };
        out.split_eqs(eqs);
        out
    }

    fn split_eqs(&mut self, eqs: &[(Type, Type)]) {
        for (a, b) in eqs {
            match (a, b) {
                (Type::Proj(base, ..), Type::Fn(sig)) if crate::traits::is_sig(a) => {
                    match &**base {
                        Type::Param(f, _) => self.sigs.push((f.clone(), sig.clone())),
                        // The signature of what isn't a type parameter: whose
                        // closure it is isn't known, so it's no one's.
                        _ => self.sigs.push((String::new(), sig.clone())),
                    }
                }
                _ => self.eqs.push((a.clone(), b.clone())),
            }
        }
    }

    /// The function's result, if it's a function's.
    pub(super) fn ret(&self) -> Option<&Type> {
        self.ret.as_ref()
    }

    /// Closure type parameter `f`'s signature, if it has one.
    pub(super) fn sig_of(&self, f: &str) -> Option<&FnType> {
        self.sigs.iter().find(|(g, _)| g == f).map(|(_, s)| s)
    }

    /// Every place type parameter `z` is (lemma P1).
    pub(super) fn occurrences(&self, cx: &Checker, z: &str) -> Vec<Occ> {
        let mut out = Vec::new();
        for (j, t) in self.params.iter().enumerate() {
            find(cx, t, z, Site::Param(j), &mut out);
        }
        if let Some(r) = &self.ret {
            find(cx, r, z, Site::Ret, &mut out);
        }
        match &self.fields {
            Some(fs) => {
                for (k, t) in fs.iter().enumerate() {
                    find(cx, t, z, Site::Field(k), &mut out);
                }
            }
            // An impl whose type's fields aren't known could hold anything.
            None if self.ret.is_none() => out.push(Occ {
                site: Site::Field(0),
                of: Of::Itself,
                shape: Shape::Hidden,
            }),
            None => {}
        }
        for (k, p) in self.preds.iter().enumerate() {
            find(cx, &p.ty, z, Site::Pred(k), &mut out);
        }
        for (k, (a, b)) in self.eqs.iter().enumerate() {
            find(cx, a, z, Site::Eq(k), &mut out);
            find(cx, b, z, Site::Eq(k), &mut out);
        }
        for (k, (_, sig)) in self.sigs.iter().enumerate() {
            for (i, p) in sig.params.iter().enumerate() {
                find(cx, p, z, Site::SigParam(k, i), &mut out);
            }
            find(cx, &sig.ret, z, Site::SigRet(k), &mut out);
        }
        out
    }

    /// The trait of bound `k`.
    fn trait_of(&self, k: usize) -> &str {
        &self.preds[k].trait_name
    }

    /// Is occurrence `o` in a bound of a marker's?
    pub(super) fn pred_is_marker(&self, o: &Occ) -> bool {
        matches!(o.site, Site::Pred(k) if marker(self.trait_of(k)))
    }

    /// Is bound `k` of an `Fn` trait's, or a marker's?
    pub(super) fn pred_is_fn_or_marker(&self, k: usize) -> bool {
        let t = self.trait_of(k);
        marker(t) || FN_TRAITS.contains(&t)
    }

    /// Is closure signature `k` closure type parameter `f`'s own?
    pub(super) fn sig_is(&self, k: usize, f: &str) -> bool {
        self.sigs[k].0 == f
    }

    /// Does an impl hold closure type parameter `g` where nothing but it
    /// can change it: in its fields, read and not written (as for an
    /// iterator, `conserved`), its bounds `Fn` traits and markers, and in
    /// no closure's signature but its own, which only takes it?
    pub(super) fn held(&self, cx: &Checker, g: &str) -> bool {
        self.occurrences(cx, g).iter().all(|o| match o.site {
            Site::Field(_) => o.of == Of::Itself && o.given(),
            Site::Pred(k) => {
                o.of == Of::Itself && o.shape == Shape::Whole && self.pred_is_fn_or_marker(k)
            }
            Site::SigParam(k, _) => self.sig_is(k, g),
            Site::Param(_) | Site::Ret | Site::Eq(_) | Site::SigRet(_) => false,
        })
    }

    /// Is equality bound `k` the blanket impl's own `X::IntoIter = X`,
    /// which `into_iter` gives back as it is?
    fn blanket_into_iter(&self, k: usize, x: &str) -> bool {
        matches!(&self.eqs[k], (Type::Proj(base, tr, n), Type::Param(q, _))
            if tr == INTO_ITERATOR
                && n == "IntoIter"
                && q == x
                && matches!(&**base, Type::Param(p, _) if p == x))
    }

    /// Is equality bound `k` one of `x`'s projections, `x::A = τ`, with
    /// `x` nowhere in `τ`? That says what an associated type of `x` is,
    /// which gives no `x`.
    fn of_x(&self, k: usize, x: &str) -> bool {
        matches!(&self.eqs[k], (Type::Proj(base, ..), b)
            if matches!(&**base, Type::Param(p, _) if p == x) && !free::mentions(b, x))
    }

    /// Is `x` an iterator the code can't make, only pass on: given it,
    /// it can give back, and leave behind a `&mut`, only one it was given,
    /// or what that one became by its protocol (lemma I3 of docs/82)?
    /// Its bounds are the protocol's traits, `IntoIterator`, markers; its
    /// items', markers; none on another type with it in; no equality but
    /// what one of its own associated types is; no closure that gives one
    /// or can put one where the code finds it; no function value or other
    /// hidden place that gives one.
    pub(super) fn conserved(&self, cx: &Checker, x: &str) -> bool {
        self.occurrences(cx, x)
            .iter()
            .all(|o| match (o.site, o.of) {
                // Given, or given back; but not as what a function value it's
                // given gives.
                (Site::Param(_), _) => o.shape != Shape::Hidden,
                (Site::Ret, _) => true,
                // Held where the code can't put another: read, not written.
                (Site::Field(_), Of::Itself) => o.given(),
                (Site::Field(_), Of::Items | Of::Other) => true,
                (Site::Pred(k), Of::Itself) => {
                    let t = self.trait_of(k);
                    o.shape == Shape::Whole
                        && (marker(t) || [ITERATOR, ITERATOR_DEI, INTO_ITERATOR].contains(&t))
                }
                (Site::Pred(k), Of::Items) => o.shape == Shape::Whole && marker(self.trait_of(k)),
                (Site::Pred(_), Of::Other) => false,
                (Site::Eq(k), _) => self.blanket_into_iter(k, x) || self.of_x(k, x),
                (Site::SigParam(..), Of::Itself | Of::Items) => o.given(),
                (Site::SigParam(..), Of::Other) => false,
                (Site::SigRet(_), _) => false,
            })
    }

    /// Do the items of iterator `x` come only from its protocol, on the
    /// `x`s the code has (lemma I3′ of docs/83)? It's `conserved`; no
    /// equality says what they are (`X: Iterator<Item = usize>` lets the
    /// code make them); none is given, nor held; no closure gives one, or
    /// can put one where the code finds it.
    pub(super) fn items_from_protocol(&self, cx: &Checker, x: &str) -> bool {
        self.conserved(cx, x)
            && self
                .occurrences(cx, x)
                .iter()
                .all(|o| match (o.site, o.of) {
                    (_, Of::Itself) => true,
                    (Site::Ret, _) => true,
                    (Site::Param(_) | Site::Field(_), Of::Items | Of::Other) => false,
                    // `conserved` saw to these.
                    (Site::Pred(_), Of::Items | Of::Other) => true,
                    (Site::Eq(k), Of::Items | Of::Other) => {
                        self.blanket_into_iter(k, x) || !matches!(o.of, Of::Items)
                    }
                    (Site::SigParam(..), Of::Items | Of::Other) => o.given(),
                    (Site::SigRet(_), Of::Items | Of::Other) => false,
                })
    }

    /// The closure type parameter that every value of type parameter `u`
    /// comes from, if one does (lemma I4 of docs/82): `u` is that one's
    /// result, whole, and anywhere else only given (to that closure, or
    /// another), bounded by markers, or given back.
    pub(super) fn closure_output(&self, cx: &Checker, u: &str) -> Option<String> {
        let mut own = None;
        for o in self.occurrences(cx, u) {
            match o.site {
                Site::SigRet(k) if o.of == Of::Itself && o.shape == Shape::Whole => {
                    if own.is_some() {
                        return None;
                    }
                    own = Some(self.sigs[k].0.clone());
                }
                Site::SigRet(_) => return None,
                Site::SigParam(..) if o.of == Of::Itself && o.given() => {}
                Site::SigParam(..) => return None,
                Site::Pred(k) => {
                    if !(o.of == Of::Itself && o.shape == Shape::Whole && marker(self.trait_of(k)))
                    {
                        return None;
                    }
                }
                Site::Ret => {}
                Site::Param(_) | Site::Field(_) | Site::Eq(_) => return None,
            }
        }
        own
    }

    /// The one parameter every value of closure type parameter `f` the
    /// code can call comes from, if there is one (lemma V5 of docs/78): a
    /// parameter whose type is `f`; `f` in no other's, in no closure's
    /// signature but its own (not as its own result), and in no equality
    /// bound; no bound on it, or a type it's in, that makes one. Two
    /// parameters of type `F`, or an `Option<F>`, or a closure giving an
    /// `F`, could each be another closure of the type, with other
    /// captures.
    pub(super) fn sole_source(&self, cx: &Checker, f: &str) -> Option<usize> {
        let mut at = None;
        for o in self.occurrences(cx, f) {
            match o.site {
                Site::Param(j) if o.of == Of::Itself && o.shape == Shape::Whole && at.is_none() => {
                    at = Some(j)
                }
                Site::Param(_) => return None,
                Site::Ret => {}
                Site::Pred(k) => {
                    let t = self.trait_of(k);
                    if !(targs::consumer(&cx.traits, t, &mut Vec::new()) || FN_TRAITS.contains(&t))
                    {
                        return None;
                    }
                }
                // Its own signature may take one, not give one.
                Site::SigParam(k, _) if self.sigs[k].0 == f => {}
                Site::SigParam(..) | Site::SigRet(_) => return None,
                Site::Eq(_) | Site::Field(_) => return None,
            }
        }
        at
    }

    /// Can a function of this signature keep the closure it's given as
    /// parameter `j`, of type parameter `f`, to be called once it's
    /// returned, with anything (lemma W7 of docs/63)? Unless `f`'s only
    /// source is that parameter (`sole_source`), and it's in neither the
    /// result nor any bound but its own `Fn` traits and markers, and the
    /// result has no `impl` type, which could hold it.
    pub(super) fn kept(&self, cx: &Checker, f: &str, j: usize) -> bool {
        if self.sole_source(cx, f) != Some(j) {
            return true;
        }
        let opaque = self.ret.as_ref().is_some_and(has_opaque);
        opaque
            || self.occurrences(cx, f).iter().any(|o| match o.site {
                Site::Ret => true,
                Site::Pred(k) => {
                    let t = self.trait_of(k);
                    !(o.of == Of::Itself
                        && o.shape == Shape::Whole
                        && (marker(t) || FN_TRAITS.contains(&t)))
                }
                Site::Param(_) | Site::SigParam(..) => false,
                Site::Field(_) | Site::Eq(_) | Site::SigRet(_) => true,
            })
    }

    /// Where values of type parameter `x` come from that a call can't
    /// follow (lemma A2 of docs/60): a bound on it that makes values (not
    /// a `consumer`, nor a `parametric_impl` at `ty`, what `x` is), or one
    /// on a type it's in that might; an equality that says what it is; or
    /// a closure's result, but a closure's that `followed` says the call
    /// follows, by its summary (lemma C2 of docs/74). `None` if none.
    ///
    /// What a closure can write back through a parameter isn't one of
    /// these: a closure given a `&mut` to what's declared of `x` must be
    /// walked keeping it (`Walk::closures_given`, lemma W7 of docs/63).
    pub(super) fn makes(
        &self,
        cx: &Checker,
        x: &str,
        ty: Option<&Type>,
        followed: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        let traits = &cx.traits;
        for o in self.occurrences(cx, x) {
            match o.site {
                Site::Pred(k) => {
                    let p = &self.preds[k];
                    let name = traits.shorten(&p.trait_name);
                    let this = matches!(&p.ty, Type::Param(q, _) if q == x);
                    if this {
                        if !targs::consumer(traits, &p.trait_name, &mut Vec::new())
                            && !ty.is_some_and(|t| targs::parametric_impl(cx, t, &p.trait_name))
                        {
                            return Some(format!("`{x}: {name}`"));
                        }
                    } else if !targs::consumer(traits, &p.trait_name, &mut Vec::new())
                        && !FN_TRAITS.contains(&p.trait_name.as_str())
                    {
                        return Some(format!("`{}: {name}`", p.ty));
                    }
                }
                Site::Eq(k) => {
                    let (a, b) = &self.eqs[k];
                    // What an impl says its trait's associated type is,
                    // for its own type, isn't a bound: a value of it is
                    // made only by the impl's methods, which can make an
                    // `x` no more than this one can.
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
                    return Some(format!("`{a} = {b}`"));
                }
                Site::SigRet(k) => {
                    let (f, sig) = &self.sigs[k];
                    if !followed(f) {
                        return Some(format!("a closure's result, `{}`", sig.ret));
                    }
                }
                // Given; given back; written back by a closure, which W7
                // checks; held.
                Site::Param(_) | Site::Ret | Site::SigParam(..) | Site::Field(_) => {}
            }
        }
        None
    }
}

/// Does `t` have an `impl` type in it, which may hold anything?
pub(super) fn has_opaque(t: &Type) -> bool {
    matches!(t, Type::Opaque(..)) || t.parts().any(has_opaque)
}

/// Do the methods of `Iterator` and `DoubleEndedIterator` take their
/// items only to give them to closures, never get one from a closure
/// (lemma I4′ of docs/83)? Then an impl overriding one gets items only
/// from its iterators' protocols, as its own bounds allow; checked of the
/// traits as they are, so a method added to them that could give one
/// leaves no adapter's items known.
pub(super) fn protocol_methods_harmless(cx: &Checker) -> bool {
    for tr in [ITERATOR, ITERATOR_DEI] {
        let Some(d) = cx.traits.traits.get(tr) else {
            return false;
        };
        for m in &d.methods {
            let sig = Signature::of_scheme(&m.scheme);
            let ok = sig
                .occurrences(cx, "Self")
                .iter()
                .all(|o| match (o.site, o.of) {
                    (Site::SigRet(_), Of::Items | Of::Itself | Of::Other) => false,
                    (Site::SigParam(..), Of::Items) => o.given(),
                    (Site::SigParam(..), Of::Itself | Of::Other) => false,
                    (Site::Eq(_), _) => false,
                    (Site::Param(_) | Site::Ret | Site::Pred(_) | Site::Field(_), _) => true,
                });
            if !ok {
                return false;
            }
        }
    }
    true
}
