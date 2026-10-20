//! Regions: working out how long each reference must be valid for, and
//! checking that the function's signature allows that.
//!
//! This is *non-lexical lifetimes* (NLL), rustc's borrow checker since 2018
//! (RFC 2094). A region is a set of points in the body, plus, for a
//! function's lifetime parameters, the *end* of each lifetime it outlives,
//! which is somewhere in the caller, after the call. Every region in every
//! type in the body gets a variable (`renumber`), and two kinds of
//! constraint decide what's in them:
//!
//! - *Liveness*: if a variable may be used later, every region in its type
//!   holds the point. A reference must be good for as long as it's around
//!   to use.
//! - *Outlives*: `'a: 'b`, "`'a` holds everything `'b` does". These come
//!   from the types: where a value of type `&'a T` goes into a place of type
//!   `&'b T`, `'a: 'b` (*subtyping*, see `relate`), and from a function's
//!   signature where it's called.
//!
//! The smallest regions that satisfy them all are found by growing them
//! until nothing changes (`solve`). Then two things are checked: that a
//! lifetime parameter `'a` only ends up holding the end of a `'b` the
//! signature says it outlives (`check_universals`), and that no loan is used
//! in a way that clashes with it while its region lasts (`loans.rs`).

use super::mir::{Body, Point, StmtKind, Term};
use crate::error::Span;
use crate::types::{AdtKind, Adts, Bound, FnType, Region, Subst, Type};
use std::collections::HashMap;

/// How a type depends on one of its parameters, which decides how
/// subtyping on the parameter becomes subtyping on the type. `&'a T` is
/// *covariant* in `'a` and `T`: a `&'long int` is a `&'short int`. `&'a mut
/// T` is *invariant* in `T`, or a `&mut &'long int` could be used to write a
/// `&'short int` where the long one is expected. A function type is
/// *contravariant* in its parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Variance {
    /// Not used at all: anything goes.
    Bi,
    Co,
    Contra,
    Inv,
}

impl Variance {
    /// The variance of something used with variance `inner`, in a place
    /// with variance `self`.
    fn then(self, inner: Variance) -> Variance {
        match (self, inner) {
            (Variance::Bi, _) | (_, Variance::Bi) => Variance::Bi,
            (Variance::Co, v) => v,
            (Variance::Contra, v) => v.flip(),
            (Variance::Inv, _) => Variance::Inv,
        }
    }

    fn flip(self) -> Variance {
        match self {
            Variance::Co => Variance::Contra,
            Variance::Contra => Variance::Co,
            v => v,
        }
    }

    /// The least that's both: what a parameter used both ways is.
    fn join(self, other: Variance) -> Variance {
        match (self, other) {
            (Variance::Bi, v) | (v, Variance::Bi) => v,
            (a, b) if a == b => a,
            _ => Variance::Inv,
        }
    }
}

/// What the borrow checker needs to know about each struct and enum: how it
/// varies with each parameter, and what its parameters must outlive for it
/// to make sense (*well-formedness*).
pub(crate) struct AdtInfo {
    /// For each, the variance of each lifetime parameter, then each type
    /// parameter.
    variances: HashMap<String, Vec<Variance>>,
    /// For each, what must hold of its parameters: `struct R<'a, T> { r:
    /// &'a T }` needs `T: 'a`. Inferred from the fields, as in Rust (RFC
    /// 2093), plus what it declares.
    pub(crate) outlives: HashMap<String, Vec<Bound>>,
}

impl AdtInfo {
    pub(crate) fn new(adts: &Adts) -> AdtInfo {
        let mut names: Vec<&String> = adts.keys().collect();
        names.sort();
        // Variances: start from "not used", and widen until nothing changes.
        let mut variances: HashMap<String, Vec<Variance>> = names
            .iter()
            .map(|n| {
                let d = &adts[*n];
                (
                    (*n).clone(),
                    vec![Variance::Bi; d.lifetimes.len() + d.params.len()],
                )
            })
            .collect();
        loop {
            let mut changed = false;
            for n in &names {
                let d = &adts[*n];
                // A closure is invariant in everything: its signature uses
                // its parameters too, which its fields don't show, maybe
                // the other way round. (Rust's are too.)
                if crate::types::is_closure(n) {
                    let inv = vec![Variance::Inv; d.lifetimes.len() + d.params.len()];
                    changed |= variances.insert((*n).clone(), inv.clone()) != Some(inv);
                    continue;
                }
                let mut v = variances[*n].clone();
                for t in field_types(adts, n) {
                    vary(&t, Variance::Co, d, &variances, &mut v);
                }
                if v != variances[*n] {
                    variances.insert((*n).clone(), v);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // Outlives requirements, the same way.
        let mut outlives: HashMap<String, Vec<Bound>> = names
            .iter()
            .map(|n| ((*n).clone(), adts[*n].bounds.clone()))
            .collect();
        loop {
            let mut changed = false;
            for n in &names {
                let mut reqs = outlives[*n].clone();
                for t in field_types(adts, n) {
                    for b in wf_bounds(&t, adts, &outlives) {
                        if !reqs.contains(&b) {
                            reqs.push(b);
                        }
                    }
                }
                if reqs.len() != outlives[*n].len() {
                    outlives.insert((*n).clone(), reqs);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        AdtInfo {
            variances,
            outlives,
        }
    }
}

/// The types of every field of every variant of struct or enum `name`, as
/// declared.
fn field_types(adts: &Adts, name: &str) -> Vec<Type> {
    match &adts[name].kind {
        AdtKind::Struct(fields) => fields.iter().map(|(_, t)| t.clone()).collect(),
        AdtKind::Enum(variants) => variants.iter().flat_map(|(_, ts)| ts.clone()).collect(),
    }
}

/// Widens `out`, the variances of `d`'s parameters, with how `t` uses
/// them, `t` being used with variance `v`.
fn vary(
    t: &Type,
    v: Variance,
    d: &crate::types::AdtDef,
    variances: &HashMap<String, Vec<Variance>>,
    out: &mut [Variance],
) {
    let region = |r: &Region, v: Variance, out: &mut [Variance]| {
        if let Region::Named(n) = r
            && let Some(i) = d.lifetimes.iter().position(|l| l == n)
        {
            out[i] = out[i].join(v);
        }
    };
    match t {
        Type::Param(p) => {
            if let Some(i) = d.params.iter().position(|q| q == p) {
                let i = d.lifetimes.len() + i;
                out[i] = out[i].join(v);
            }
        }
        Type::Ref(r, m, inner) => {
            region(r, v, out);
            let inner_v = if *m { Variance::Inv } else { Variance::Co };
            vary(inner, v.then(inner_v), d, variances, out);
        }
        Type::Box(inner) => vary(inner, v, d, variances, out),
        Type::Fn(sig) => {
            for p in &sig.params {
                vary(p, v.then(Variance::Contra), d, variances, out);
            }
            vary(&sig.ret, v, d, variances, out);
            vary(&sig.effects, Variance::Inv, d, variances, out);
        }
        // An effect set has no regions but those of the associated effects
        // in it, which are projections.
        Type::Effects(items) => {
            for i in items {
                vary(i, Variance::Inv, d, variances, out);
            }
        }
        Type::Adt(name, regions, args) => {
            let vs = &variances[name];
            for (i, r) in regions.iter().enumerate() {
                region(r, v.then(vs[i]), out);
            }
            for (i, a) in args.iter().enumerate() {
                vary(a, v.then(vs[regions.len() + i]), d, variances, out);
            }
        }
        // What a projection stands for may use its type's parameters any
        // way at all: `<W<'a> as Tr>::Out` can be a `&'a mut &'a int`
        // though `W` is covariant. So it's invariant in them.
        Type::Proj(inner, ..) => vary(inner, Variance::Inv, d, variances, out),
        // How an opaque type's hidden type uses its parameters isn't for
        // anything else to know, so it's invariant in them all.
        Type::Opaque(_, regions, args) => {
            for r in regions {
                region(r, Variance::Inv, out);
            }
            for a in args {
                vary(a, Variance::Inv, d, variances, out);
            }
        }
        Type::Int
        | Type::Bool
        | Type::Unit
        | Type::Never
        | Type::Var(_)
        | Type::Alloc
        | Type::Panic => {}
    }
}

/// What must hold for `t` to make sense: for `&'a T`, everything in `T`
/// must outlive `'a`, since a reference can't outlive what it points to;
/// for a struct or enum, what it requires of its arguments. Function types
/// require nothing: a function value is code, valid forever, whatever its
/// type mentions.
pub(crate) fn wf_bounds(
    t: &Type,
    adts: &Adts,
    outlives: &HashMap<String, Vec<Bound>>,
) -> Vec<Bound> {
    let mut out = Vec::new();
    wf_into(t, adts, outlives, &mut out);
    out
}

fn wf_into(t: &Type, adts: &Adts, outlives: &HashMap<String, Vec<Bound>>, out: &mut Vec<Bound>) {
    match t {
        Type::Ref(r, _, inner) => {
            type_outlives(inner, r, out);
            wf_into(inner, adts, outlives, out);
        }
        Type::Box(inner) => wf_into(inner, adts, outlives, out),
        Type::Adt(name, regions, args) => {
            let d = &adts[name];
            let s = Subst::new(&d.params, args).with_regions(&d.lifetimes, regions);
            for b in outlives.get(name).into_iter().flatten() {
                match b {
                    Bound::Region(a, b) => push(out, Bound::Region(s.region(a), s.region(b))),
                    Bound::Param(p, r) => {
                        type_outlives(&s.apply(&Type::Param(p.clone())), &s.region(r), out)
                    }
                    Bound::Proj(p, r) => {
                        type_outlives(&adts.normalise(&s.apply(p)), &s.region(r), out)
                    }
                }
            }
            for a in args {
                wf_into(a, adts, outlives, out);
            }
        }
        Type::Proj(inner, ..) => wf_into(inner, adts, outlives, out),
        // What its hidden type needs was checked where that's known, and
        // holds wherever the type is made, by a call of the function.
        Type::Opaque(_, _, args) => {
            for a in args {
                wf_into(a, adts, outlives, out);
            }
        }
        // Effects are no values, and hold no references.
        Type::Fn(_)
        | Type::Effects(_)
        | Type::Alloc
        | Type::Panic
        | Type::Int
        | Type::Bool
        | Type::Unit
        | Type::Never
        | Type::Param(_)
        | Type::Var(_) => {}
    }
}

fn push(out: &mut Vec<Bound>, b: Bound) {
    if !out.contains(&b) {
        out.push(b);
    }
}

/// `t: 'r`, as what it means for the regions and type parameters in `t`:
/// each must outlive `'r`. Not through function types (see `wf_bounds`).
pub(crate) fn type_outlives(t: &Type, r: &Region, out: &mut Vec<Bound>) {
    match t {
        Type::Ref(q, _, inner) => {
            push(out, Bound::Region(q.clone(), r.clone()));
            type_outlives(inner, r, out);
        }
        // An opaque type's hidden type only has regions it's at, and
        // `'static`, and only the types it's at in it (see
        // `Builder::member_constraints`), so it outlives what they all do.
        Type::Adt(_, regions, args) | Type::Opaque(_, regions, args) => {
            for q in regions {
                push(out, Bound::Region(q.clone(), r.clone()));
            }
            for a in args {
                type_outlives(a, r, out);
            }
        }
        Type::Param(p) => push(out, Bound::Param(p.clone(), r.clone())),
        // A rigid projection's regions are its type's and `'static` (its
        // impl's associated type can only mention the impl's parameters and
        // `'static`), so it outlives what its type does; or what a bound
        // says it does: a test of its own (see `Regions::subject`).
        Type::Proj(inner, ..) if matches!(**inner, Type::Param(_) | Type::Proj(..)) => {
            push(out, Bound::Proj(t.clone(), r.clone()))
        }
        Type::Proj(inner, ..) => type_outlives(inner, r, out),
        Type::Box(inner) => type_outlives(inner, r, out),
        // Nor do effects, as an argument of a closure's type.
        Type::Fn(_)
        | Type::Effects(_)
        | Type::Alloc
        | Type::Panic
        | Type::Int
        | Type::Bool
        | Type::Unit
        | Type::Never
        | Type::Var(_) => {}
    }
}

/// `'sup: 'sub`, because of something at `span`.
#[derive(Debug, Clone)]
pub(crate) struct Outlives {
    pub(crate) sup: usize,
    pub(crate) sub: usize,
    pub(crate) span: Span,
}

/// The regions of one body: its variables and what's known about them.
///
/// Variable 0 is `'static`, then come the *universal* regions: the
/// function's lifetime parameters, which hold the whole body and the end of
/// their lifetime, in the caller. The rest are the body's own, but for
/// *placeholders*, which are universal too: the lifetimes a call of a
/// higher-ranked function type picks, which the body can't know anything
/// about (see `relate_fns`).
pub(crate) struct Regions {
    /// What the universal regions are, `'static` first, then the
    /// signature's, then placeholders.
    pub(crate) universal: Vec<Region>,
    /// The variable of each universal region.
    pub(crate) universal_vars: Vec<usize>,
    /// How many universal regions the signature has, `'static` included:
    /// those after are placeholders.
    pub(crate) signature: usize,
    /// How many variables there are.
    pub(crate) count: usize,
    pub(crate) constraints: Vec<Outlives>,
    /// `T: 'r`: type parameter `T`, or rigid projection `T::Out`, must
    /// outlive region variable `'r`.
    pub(crate) type_tests: Vec<(Type, usize, Span)>,
    /// `known[a][b]`: universal `a` is known to outlive universal `b`, from
    /// what the signature says. Universal regions are numbered by their
    /// place in `universal`.
    pub(crate) known: Vec<Vec<bool>>,
    /// The universal regions each type parameter, or rigid projection, is
    /// known to outlive, from what the signature says.
    pub(crate) known_params: HashMap<Type, Vec<usize>>,
    /// In a closure's body, which universal region is the call's, if any.
    pub(crate) call: Option<usize>,
}

impl Regions {
    /// The regions of a body with lifetime parameters `lifetimes`, which
    /// are known to outlive each other as `bounds` say.
    pub(crate) fn new(lifetimes: &[String], bounds: &[Bound]) -> Regions {
        let mut universal = vec![Region::Static];
        universal.extend(lifetimes.iter().cloned().map(Region::Named));
        let n = universal.len();
        let mut r = Regions {
            count: n,
            known: vec![vec![false; n]; n],
            universal,
            universal_vars: (0..n).collect(),
            signature: n,
            constraints: Vec::new(),
            type_tests: Vec::new(),
            known_params: HashMap::new(),
            call: None,
        };
        for a in 0..n {
            r.known[a][a] = true;
            r.known[0][a] = true;
        }
        for b in bounds {
            match b {
                Bound::Region(a, b) => {
                    let (a, b) = (r.universal_var(a), r.universal_var(b));
                    r.known[a][b] = true;
                }
                Bound::Param(p, b) => {
                    let b = r.universal_var(b);
                    r.known_params
                        .entry(Type::Param(p.clone()))
                        .or_default()
                        .push(b);
                }
                Bound::Proj(p, b) => {
                    let b = r.universal_var(b);
                    r.known_params.entry(p.clone()).or_default().push(b);
                }
            }
        }
        // What's known is transitive.
        for k in 0..n {
            for a in 0..n {
                for b in 0..n {
                    if r.known[a][k] && r.known[k][b] {
                        r.known[a][b] = true;
                    }
                }
            }
        }
        for outlived in r.known_params.values_mut() {
            let mut all: Vec<usize> = (0..n)
                .filter(|b| outlived.iter().any(|a| r.known[*a][*b]))
                .collect();
            all.sort();
            *outlived = all;
        }
        r
    }

    /// The universal regions type parameter or rigid projection `t` is
    /// known to outlive: what the signature says of it, and, for
    /// `T::Out`, what it says of `T`, as a projection's regions are its
    /// type's and `'static` (Rust's RFC 1214).
    pub(crate) fn known_of(&self, t: &Type) -> Vec<usize> {
        let mut out = self.known_params.get(t).cloned().unwrap_or_default();
        if let Type::Proj(inner, ..) = t {
            for u in self.known_of(inner) {
                if !out.contains(&u) {
                    out.push(u);
                }
            }
        }
        out
    }

    /// The variable of a universal region, as signatures name them.
    pub(crate) fn universal_var(&self, r: &Region) -> usize {
        match r {
            Region::Var(v) => *v,
            r => {
                let u = self.universal.iter().position(|u| u == r).unwrap_or(0);
                self.universal_vars[u]
            }
        }
    }

    /// The region of a call of the closure whose body this is, for which
    /// it has itself: universal, like a placeholder, but the one of the
    /// call, which nothing the closure gives out may need (see
    /// `borrowck::closure`).
    pub(crate) fn call_region(&mut self) -> usize {
        let v = self.placeholder();
        self.call = Some(self.universal.len() - 1);
        v
    }

    /// A new placeholder region: universal, known only to be outlived by
    /// `'static`.
    pub(crate) fn placeholder(&mut self) -> usize {
        let v = self.fresh();
        let u = self.universal.len();
        self.universal.push(Region::Named(format!("'^p{u}")));
        self.universal_vars.push(v);
        for row in &mut self.known {
            row.push(false);
        }
        self.known.push(vec![false; u + 1]);
        self.known[u][u] = true;
        self.known[0][u] = true;
        v
    }

    pub(crate) fn fresh(&mut self) -> usize {
        self.count += 1;
        self.count - 1
    }

    /// `t` with each region a new variable, except the universal ones,
    /// which are what they are.
    pub(crate) fn renumber(&mut self, t: &Type) -> Type {
        t.map_regions(&mut |r| {
            Region::Var(match r {
                Region::Named(_) | Region::Static if self.universal.contains(r) => {
                    self.universal_var(r)
                }
                Region::Var(v) => *v,
                _ => self.fresh(),
            })
        })
    }

    /// `t` with each universal region, as signatures name them, its
    /// variable. It must have no other regions but variables.
    pub(crate) fn universal_regions(&self, t: &Type) -> Type {
        t.map_regions(&mut |r| match r {
            Region::Var(_) => r.clone(),
            r if self.universal.contains(r) => Region::Var(self.universal_var(r)),
            r => unreachable!("region `{r}` isn't universal here"),
        })
    }

    /// `t` with every region a new variable, whatever it was.
    pub(crate) fn fresh_regions(&mut self, t: &Type) -> Type {
        t.map_regions(&mut |_| Region::Var(self.fresh()))
    }

    pub(crate) fn outlives(&mut self, sup: &Region, sub: &Region, span: Span) {
        let (sup, sub) = (var(sup), var(sub));
        if sup != sub {
            self.constraints.push(Outlives { sup, sub, span });
        }
    }

    /// What `bounds` say, as constraints, for regions numbered already.
    pub(crate) fn require(&mut self, bounds: &[Bound], span: Span) {
        for b in bounds {
            match b {
                Bound::Region(a, b) => self.outlives(a, b, span),
                Bound::Param(p, r) => self.type_tests.push((Type::Param(p.clone()), var(r), span)),
                Bound::Proj(p, r) => self.type_tests.push((p.clone(), var(r), span)),
            }
        }
    }

    /// Constraints for `a` to be usable as a `b`: a value of type `a` going
    /// into a place of type `b`, where they're the same type but for their
    /// regions. `a <: b`, *subtyping*.
    pub(crate) fn subtype(&mut self, adts: &AdtInfo, a: &Type, b: &Type, span: Span) {
        self.relate(adts, a, b, Variance::Co, span);
    }

    /// Constraints for `a` and `b` to be the same type, regions and all.
    pub(crate) fn equate(&mut self, adts: &AdtInfo, a: &Type, b: &Type, span: Span) {
        self.relate(adts, a, b, Variance::Inv, span);
    }

    fn relate(&mut self, adts: &AdtInfo, a: &Type, b: &Type, v: Variance, span: Span) {
        let region = |this: &mut Regions, ra: &Region, rb: &Region, v: Variance| match v {
            Variance::Co => this.outlives(ra, rb, span),
            Variance::Contra => this.outlives(rb, ra, span),
            Variance::Inv => {
                this.outlives(ra, rb, span);
                this.outlives(rb, ra, span);
            }
            Variance::Bi => {}
        };
        match (a, b) {
            (Type::Ref(ra, m, ta), Type::Ref(rb, _, tb)) => {
                region(self, ra, rb, v);
                let inner = if *m { Variance::Inv } else { Variance::Co };
                self.relate(adts, ta, tb, v.then(inner), span);
            }
            (Type::Box(ta), Type::Box(tb)) => self.relate(adts, ta, tb, v, span),
            (Type::Fn(f), Type::Fn(g)) => match v {
                Variance::Co => self.relate_fns(adts, f, g, span),
                Variance::Contra => self.relate_fns(adts, g, f, span),
                Variance::Inv => {
                    self.relate_fns(adts, f, g, span);
                    self.relate_fns(adts, g, f, span);
                }
                Variance::Bi => {}
            },
            (Type::Adt(name, ras, tas), Type::Adt(_, rbs, tbs)) => {
                let vs = adts.variances[name].clone();
                for (i, (ra, rb)) in ras.iter().zip(rbs).enumerate() {
                    region(self, ra, rb, v.then(vs[i]));
                }
                for (i, (ta, tb)) in tas.iter().zip(tbs).enumerate() {
                    self.relate(adts, ta, tb, v.then(vs[ras.len() + i]), span);
                }
            }
            // Invariant in everything: see `vary`.
            (Type::Opaque(_, ras, tas), Type::Opaque(_, rbs, tbs)) => {
                for (ra, rb) in ras.iter().zip(rbs) {
                    region(self, ra, rb, Variance::Inv);
                }
                for (ta, tb) in tas.iter().zip(tbs) {
                    self.relate(adts, ta, tb, Variance::Inv, span);
                }
            }
            // No regions to relate. A `!` goes anywhere, having no values.
            (Type::Int, Type::Int)
            | (Type::Bool, Type::Bool)
            | (Type::Unit, Type::Unit)
            | (Type::Never, _)
            | (_, Type::Never) => {}
            (Type::Param(p), Type::Param(q)) if p == q => {}
            // Effects are no values, and borrow nothing: the checker saw
            // that one set fits in the other, which is all there is to it.
            (Type::Effects(_), Type::Effects(_)) => {}
            // Two rigid projections are the same type if their types are,
            // regions and all: invariantly (see `vary`).
            (Type::Proj(ta, x, n), Type::Proj(tb, y, m)) if x == y && n == m => {
                self.relate(adts, ta, tb, v.then(Variance::Inv), span)
            }
            // Relating two types that aren't the same up to regions would
            // say nothing of the regions in their parts that differ, and
            // so allow what it shouldn't: wherever a value goes, the borrow
            // checker must see it at the type it has there.
            (a, b) => unreachable!("relating `{a}` to `{b}`"),
        }
    }

    /// Constraints for a function of type `sub` to be usable as one of type
    /// `sup`: it must take whatever `sup` would be given, and give what
    /// `sup` would. If `sup` is higher-ranked, for every lifetime it's
    /// generic over: each is a new placeholder, which nothing is known
    /// about, so if the constraints need it to outlive anything, or
    /// anything but a variable of the body's own to outlive it, `sub` isn't
    /// general enough, and checking the universal regions finds that. If
    /// `sub` is higher-ranked, it can be used at any lifetimes, so each gets
    /// a new variable, to be worked out.
    fn relate_fns(&mut self, adts: &AdtInfo, sub: &FnType, sup: &FnType, span: Span) {
        let sup_r: Vec<Region> = sup
            .bound
            .iter()
            .map(|_| Region::Var(self.placeholder()))
            .collect();
        let sub_r: Vec<Region> = sub
            .bound
            .iter()
            .map(|_| Region::Var(self.fresh()))
            .collect();
        let (sub_params, sub_ret) = sub.instantiate(&sub_r);
        let (sup_params, sup_ret) = sup.instantiate(&sup_r);
        for (p, q) in sub_params.iter().zip(&sup_params) {
            self.relate(adts, p, q, Variance::Contra, span);
        }
        self.relate(adts, &sub_ret, &sup_ret, Variance::Co, span);
    }

    /// Constraints for `t` to make sense (see `wf_bounds`).
    pub(crate) fn well_formed(&mut self, adts: &Adts, info: &AdtInfo, t: &Type, span: Span) {
        let bounds = wf_bounds(t, adts, &info.outlives);
        self.require(&bounds, span);
    }

    /// Constraints for everything in `t` to outlive region `r`.
    pub(crate) fn type_outlives(&mut self, t: &Type, r: &Region, span: Span) {
        let mut bounds = Vec::new();
        type_outlives(t, r, &mut bounds);
        self.require(&bounds, span);
    }
}

/// The variable number of a numbered region.
fn var(r: &Region) -> usize {
    match r {
        Region::Var(v) => *v,
        Region::Static => 0,
        r => unreachable!("region `{r:?}` wasn't numbered"),
    }
}

/// A set of points and universal ends, as bits.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Set(Vec<u64>);

impl Set {
    fn new(n: usize) -> Set {
        Set(vec![0; n.div_ceil(64)])
    }

    pub(crate) fn contains(&self, i: usize) -> bool {
        self.0[i / 64] & (1 << (i % 64)) != 0
    }

    fn insert(&mut self, i: usize) -> bool {
        let had = self.contains(i);
        self.0[i / 64] |= 1 << (i % 64);
        !had
    }

    /// Adds everything in `other`; says whether that was anything new.
    fn union(&mut self, other: &Set) -> bool {
        let mut changed = false;
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            let new = *a | *b;
            changed |= new != *a;
            *a = new;
        }
        changed
    }
}

/// What each region turned out to hold.
pub(crate) struct Solution {
    pub(crate) values: Vec<Set>,
    /// Where each block's points start, numbering them all in order.
    starts: Vec<usize>,
    points: usize,
}

impl Solution {
    pub(crate) fn index(&self, p: Point) -> usize {
        self.starts[p.block.0] + p.index
    }

    /// Does region `r` hold point `p`?
    pub(crate) fn holds(&self, r: usize, p: Point) -> bool {
        self.values[r].contains(self.index(p))
    }

    /// The universal regions whose end `r` holds.
    pub(crate) fn ends(&self, r: usize, universals: usize) -> Vec<usize> {
        (0..universals)
            .filter(|u| self.values[r].contains(self.points + u))
            .collect()
    }
}

/// Which locals are *live* before each point: may be used after it, before
/// they're given a new value. A local whose memory goes away doesn't count
/// as used then: dropping a value never looks at the references in it.
pub(crate) fn liveness(body: &Body) -> HashMap<Point, Vec<bool>> {
    let n = body.locals.len();
    let preds = body.predecessors();
    let mut live_in: Vec<Vec<Vec<bool>>> = body
        .blocks
        .iter()
        .map(|b| vec![vec![false; n]; b.stmts.len() + 1])
        .collect();
    let mut todo: Vec<usize> = (0..body.blocks.len()).collect();
    while let Some(b) = todo.pop() {
        let block = &body.blocks[b];
        // Live at the end: live at the start of any successor.
        let mut live = vec![false; n];
        for s in body.successors(super::mir::Bb(b)) {
            for (l, x) in live_in[s.0][0].iter().enumerate() {
                live[l] |= x;
            }
        }
        if let Some(Term::Branch(o, ..)) = &block.term
            && let Some(p) = o.place()
        {
            live[p.local.0] = true;
        }
        if let Some(Term::Return) = &block.term {
            live[0] = true;
        }
        let mut changed = live_in[b][block.stmts.len()] != live;
        live_in[b][block.stmts.len()] = live.clone();
        for (i, s) in block.stmts.iter().enumerate().rev() {
            match &s.kind {
                StmtKind::Assign { place, value, .. } => {
                    // Giving a local a whole new value ends its old one's
                    // life. Writing to part of it, or through it, doesn't,
                    // and is taken as using it.
                    live[place.local.0] = !place.proj.is_empty();
                    for o in value.operands() {
                        if let Some(p) = o.place() {
                            live[p.local.0] = true;
                        }
                    }
                    if let super::mir::Rvalue::Ref(_, _, p) = value {
                        live[p.local.0] = true;
                    }
                }
                StmtKind::Inspect(p) | StmtKind::Consume(p, _) => live[p.local.0] = true,
                StmtKind::Dead(l) => live[l.0] = false,
            }
            changed |= live_in[b][i] != live;
            live_in[b][i] = live.clone();
        }
        if changed {
            todo.extend(preds[b].iter().map(|p| p.0));
        }
    }
    let mut out = HashMap::new();
    for p in body.points() {
        out.insert(p, live_in[p.block.0][p.index].clone());
    }
    out
}

impl Regions {
    /// Finds the smallest regions that satisfy every constraint, with each
    /// region in a live local's type holding the points it's live at.
    pub(crate) fn solve(&self, body: &Body) -> Solution {
        let mut starts = Vec::new();
        let mut points = 0;
        for b in &body.blocks {
            starts.push(points);
            points += b.stmts.len() + 1;
        }
        let size = points + self.universal.len();
        let mut values = vec![Set::new(size); self.count];
        // Universal regions hold every point, and the end of themselves;
        // `'static` holds every end.
        for (u, &v) in self.universal_vars.iter().enumerate() {
            let value = &mut values[v];
            for p in 0..points {
                value.insert(p);
            }
            value.insert(points + u);
            if u == 0 {
                for e in 0..self.universal.len() {
                    value.insert(points + e);
                }
            }
        }
        let live = liveness(body);
        for p in body.points() {
            let i = starts[p.block.0] + p.index;
            for (l, is_live) in live[&p].iter().enumerate() {
                if *is_live {
                    for r in body.locals[l].ty.regions() {
                        values[var(r)].insert(i);
                    }
                }
            }
        }
        loop {
            let mut changed = false;
            for c in &self.constraints {
                if c.sup == c.sub {
                    continue;
                }
                let sub = values[c.sub].clone();
                changed |= values[c.sup].union(&sub);
            }
            if !changed {
                break;
            }
        }
        Solution {
            values,
            starts,
            points,
        }
    }

    /// The constraints that lead from region variable `from` to one
    /// holding the end of universal region `end` from the start: why `from`
    /// had to grow to hold it. In order, from `from`.
    pub(crate) fn blame(&self, from: usize, end: usize, sol: &Solution) -> Vec<&Outlives> {
        let mut parent: HashMap<usize, usize> = HashMap::new();
        let mut todo = std::collections::VecDeque::from([from]);
        let mut seen = vec![false; self.count];
        seen[from] = true;
        while let Some(r) = todo.pop_front() {
            if r == self.universal_vars[end] || (r == 0 && from != 0) {
                let mut path = Vec::new();
                let mut at = r;
                while let Some(&c) = parent.get(&at) {
                    path.push(&self.constraints[c]);
                    at = self.constraints[c].sup;
                }
                path.reverse();
                return path;
            }
            for (i, c) in self.constraints.iter().enumerate() {
                if c.sup == r && !seen[c.sub] && sol.values[c.sub].contains(sol.points + end) {
                    seen[c.sub] = true;
                    parent.insert(c.sub, i);
                    todo.push_back(c.sub);
                }
            }
        }
        Vec::new()
    }
}
