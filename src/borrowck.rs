//! The borrow checker: that a program uses memory correctly, which types
//! alone can't say. Nothing is used after it's moved, nothing borrowed is
//! changed or moved or freed while the borrow lasts, nothing is changed
//! through a `&`, and no reference outlives what it points to. See
//! docs/05-ownership.md.
//!
//! It runs on each function once its types are known (`check`), and works
//! on a lowered form of its body, MIR (`mir.rs`, `build.rs`), the way
//! rustc's does: as dataflow over a control-flow graph, so that a borrow
//! lasts only as long as it's used, not to the end of its block
//! (*non-lexical lifetimes*).
//!
//! - `moves.rs`: nothing is used after it's moved, or moved out from behind
//!   a reference, and only what's `mut` is changed.
//! - `regions.rs`: how long each reference must be valid for, and that the
//!   signature allows it.
//! - `loans.rs`: nothing borrowed is used in a way that clashes with the
//!   borrow while it lasts.
//!
//! A function that returns an opaque type, `impl Fn(i64) -> i64`, returns
//! the type it stands for, with regions of its own, each of which must be
//! one of the function's lifetimes (`member_constraints`).
//!
//! A closure is checked as a function of its own, whose signature's regions
//! are its *universal* regions. What it needs of them, it can't decide: the
//! function it's in does, so it's passed up as constraints on the
//! closure's type there.

mod build;
mod loans;
mod mir;
mod moves;
mod regions;

use crate::ast::*;
use crate::check::{Checker, is_result_lifetime, result_span};
use crate::error::{Edit, Error, Fix, Lint, Span, Warning};
use crate::traits::FnKind;
use crate::types::{FnType, Region, Scheme, Type};
use build::{Builder, ClosureUse};
use mir::{EnvVar, LocalKind, Place, Proj};
use regions::{AdtInfo, Regions, wf_bounds};
use std::collections::{HashMap, HashSet};

/// Checks function `f`, whose type is `scheme`, and returns the warnings
/// about it.
pub(crate) fn check(cx: &Checker, f: &FnDecl, scheme: &Scheme) -> Result<Vec<Warning>, Error> {
    let info = AdtInfo::new(cx.adts());
    let Type::Fn(sig) = &scheme.ty else {
        unreachable!("a function has a function type")
    };
    // What its signature takes for granted: what it declares, and what its
    // types need to make sense (*implied bounds*). Callers check those.
    let mut bounds = scheme.bounds.clone();
    for t in sig.params.iter().chain([&*sig.ret]) {
        bounds.extend(wf_bounds(t, cx.adts(), &info.outlives));
    }
    let regions = Regions::new(&scheme.lifetimes, &bounds);
    let adts = local_adts(cx, &regions);
    let mut b = Builder::new(cx, &adts, &info, regions);
    // Its body returns the types its opaque types stand for, with regions
    // of their own: each must turn out to be one of the function's
    // lifetimes, or `'static`, as that's all an opaque type says it may
    // borrow for (`member_constraints`).
    let ret = b.regions.renumber(&reveal_own(&sig.ret, cx));
    let mut hidden = Vec::new();
    pair_opaques(&sig.ret, &ret, &mut hidden);
    for (symbol, h) in &hidden {
        let d = &cx.traits().opaques[symbol];
        let what = crate::types::opaque_text(symbol).to_string();
        for r in outside_fns(h) {
            if let Region::Var(v) = r
                && b.regions.universal_vars.iter().all(|u| u != v)
            {
                b.members.push((*v, d.span, what.clone()));
            }
        }
        // What its equality bounds say, with regions: a closure that
        // returns `&'a i64` isn't an `impl Fn() -> &'static i64`.
        let generic = d.generic(symbol);
        for (lhs, rhs) in &d.eqs {
            let lhs = adts.normalise(&replace_type(lhs, &generic, h));
            let rhs = b.regions.renumber(rhs);
            b.regions.equate(&info, &lhs, &rhs, d.span);
        }
    }
    let params: Vec<(String, NodeId, Type)> = f
        .params
        .iter()
        .zip(&sig.params)
        .map(|(p, t)| (p.name.name.clone(), p.name.id, b.regions.renumber(t)))
        .collect();
    b.signature(ret, &params, None);
    b.lower(&f.body);
    let names = Names::of(f, sig);
    let (warnings, _) = analyze(&mut b, &names, false)?;
    Ok(warnings)
}

/// `t`, a function's return type, with each opaque type in it the type it
/// stands for, regions erased, as the function's body sees it.
fn reveal_own(t: &Type, cx: &Checker) -> Type {
    match t {
        Type::Opaque(symbol, ..) => {
            let hidden = cx.traits().opaques[symbol].hidden.as_ref();
            hidden.expect("known once the body's types are").erased()
        }
        t => t.map(|p| reveal_own(p, cx)),
    }
}

/// Each opaque type in `written`, with what's in its place in `revealed`,
/// which is `written` with each the type it stands for.
fn pair_opaques(written: &Type, revealed: &Type, out: &mut Vec<(String, Type)>) {
    match written {
        Type::Opaque(symbol, ..) => out.push((symbol.clone(), revealed.clone())),
        t => {
            for (w, r) in t.parts().zip(revealed.parts()) {
                pair_opaques(w, r, out);
            }
        }
    }
}

/// The regions of `t` but those in function types: a function value is
/// code, which holds no references, so it's good for as long as anything,
/// whatever its type mentions (see `regions::type_outlives`).
fn outside_fns(t: &Type) -> Vec<&Region> {
    let mut out = Vec::new();
    match t {
        Type::Fn(_) => return out,
        Type::Ref(r, ..) => out.push(r),
        Type::Adt(_, rs, _) | Type::Opaque(_, rs, _) => out.extend(rs),
        _ => {}
    }
    for p in t.parts() {
        out.extend(outside_fns(p));
    }
    out
}

/// `t`, with every `from` in it replaced by `to`.
fn replace_type(t: &Type, from: &Type, to: &Type) -> Type {
    match t == from {
        true => to.clone(),
        false => t.map(|p| replace_type(p, from, to)),
    }
}

/// Makes each region of the types a function's opaque types stand for,
/// `b.members`, one of the function's lifetimes, or `'static`: the least
/// of those that holds all it must, and is held by all that must hold it,
/// given the constraints so far. That's rustc's *member constraints*.
///
/// It's only inference: which one is picked decides which programs are
/// accepted, not whether what's accepted is sound, as the region is then
/// made equal to it, and everything is checked again with that.
/// Soundness needs only that it's equal to one: then the type holds no
/// reference that lasts less than what the opaque type is at, which is
/// what `type_outlives` takes for granted of it, and what keeps the loans
/// it holds alive wherever it goes.
fn member_constraints(b: &mut Builder, names: &Names) -> Result<(), Error> {
    let signature = b.regions.signature;
    let universals = b.regions.universal.len();
    for (h, span, what) in b.members.clone() {
        let sol = b.regions.solve(&b.body);
        // What it must hold: the ends of these.
        let lower = sol.ends(h, universals);
        // What must hold it: the universal regions it's reached from.
        let mut seen = vec![false; b.regions.count];
        let mut todo: Vec<usize> = Vec::new();
        let mut upper = Vec::new();
        for u in 0..signature {
            seen.iter_mut().for_each(|s| *s = false);
            todo.clear();
            todo.push(b.regions.universal_vars[u]);
            while let Some(r) = todo.pop() {
                if r == h {
                    upper.push(u);
                    break;
                }
                for c in &b.regions.constraints {
                    if c.sup == r && !seen[c.sub] {
                        seen[c.sub] = true;
                        todo.push(c.sub);
                    }
                }
            }
        }
        let known = &b.regions.known;
        let fits: Vec<usize> = (0..signature)
            .filter(|&c| lower.iter().all(|&e| e < signature && known[c][e]))
            .filter(|&c| upper.iter().all(|&u| known[u][c]))
            .collect();
        let Some(&c) = fits.iter().find(|&&c| fits.iter().all(|&o| known[o][c])) else {
            let mut own: Vec<String> = (1..signature).map(|u| names.lifetime(b, u)).collect();
            own.push("`'static`".to_string());
            return Err(Error::ty(
                span,
                format!(
                    "the type behind `{what}` borrows for a lifetime that isn't one of {}: \
                     those are all it can borrow for, and nothing says which one this is",
                    or_list(&own)
                ),
            ));
        };
        let u = Region::Var(b.regions.universal_vars[c]);
        b.regions.outlives(&Region::Var(h), &u, span);
        b.regions.outlives(&u, &Region::Var(h), span);
    }
    Ok(())
}

/// `a`, `a or b`, `a, b or c`.
fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The checker's structs and enums, with what the equality bounds in scope
/// say numbered as the regions of a body with `regions`: `T::Out` for `T:
/// Tr<Out = &'a i64>` normalises to `&'a i64`, with `'a` the body's
/// universal region. The checker keeps them as written, regions erased or
/// named, which the borrow checker can't relate.
fn local_adts(cx: &Checker, regions: &Regions) -> crate::types::Adts {
    let mut adts = cx.adts().clone();
    adts.proj.local = adts
        .proj
        .local
        .iter()
        .map(|(lhs, rhs)| (lhs.clone(), regions.universal_regions(rhs)))
        .collect();
    adts
}

/// What a closure needs of the regions in its signature, which the
/// function it's in has to see to: that universal region `.0` outlives
/// universal `.1`, or that type parameter `.0` outlives universal `.1`.
/// And the variables it captured by value and changes, by id, which must
/// be `mut` where it is.
#[derive(Default)]
struct Requirements {
    regions: Vec<(usize, usize)>,
    params: Vec<(Type, usize)>,
    muts: Vec<NodeId>,
}

/// Checks the closures in a lowered body, and adds what each needs of its
/// regions to its constraints. Returns the warnings and errors about them,
/// and the variables they captured by value and change, by id.
fn closures_in(b: &mut Builder) -> (Vec<Warning>, Vec<Error>, Vec<NodeId>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut muts = Vec::new();
    for c in std::mem::take(&mut b.closures) {
        match closure(b, &c) {
            Ok((w, reqs)) => {
                warnings.extend(w);
                let regions = c.ty.regions().into_iter().cloned().collect::<Vec<_>>();
                let external = b.regions.signature - 1;
                let vars = b.regions.universal_vars.clone();
                let region = |u: usize| match u {
                    0 => Region::Static,
                    u if u <= external => Region::Var(vars[u]),
                    u => regions[u - 1 - external].clone(),
                };
                for (u, e) in reqs.regions {
                    b.regions.outlives(&region(u), &region(e), c.span);
                }
                muts.extend(reqs.muts);
                for (p, e) in reqs.params {
                    let Region::Var(v) = region(e) else {
                        b.regions.type_tests.push((p, 0, c.span));
                        continue;
                    };
                    b.regions.type_tests.push((p, v, c.span));
                }
            }
            Err(e) => errors.push(e),
        }
    }
    (warnings, errors, muts)
}

/// Checks a lowered body, its closures first. For a closure, `propagate`:
/// what it needs of its universal regions is returned rather than an error.
fn analyze(
    b: &mut Builder,
    names: &Names,
    propagate: bool,
) -> Result<(Vec<Warning>, Requirements), Error> {
    let (mut warnings, mut errors, muts) = closures_in(b);
    let (mut needed, captured) = match moves::behind_references(b)
        .and_then(|()| moves::abstraction(b))
        .and_then(|()| moves::mutability(b))
    {
        Ok(needed) => needed,
        Err(e) => {
            errors.push(e);
            (Vec::new(), Vec::new())
        }
    };
    let mut reqs = Requirements {
        muts: captured,
        ..Requirements::default()
    };
    // What closures in it change of what they captured by value must be
    // `mut` here, or where this captured it from.
    for id in muts {
        match b.var_local(id) {
            Some(l) => needed.push(l),
            None => reqs.muts.push(id),
        }
    }
    // What's used up is only worth asking once what's moved is right.
    if let Err(e) = moves::moves(b).and_then(|()| moves::linear(b)) {
        errors.push(e);
    }
    b.body.drop_uses = moves::drop_uses(b);
    // That a region of an opaque type's can't be one of the function's
    // lifetimes is often because of something else that's wrong, which is
    // what's worth saying, if there's anything.
    let members = member_constraints(b, names).err();
    let sol = b.regions.solve(&b.body);
    if let Err(e) = loans::check(b, &sol) {
        errors.push(e);
    }
    let universals = b.regions.universal.len();
    let signature = b.regions.signature;
    for u in 1..universals {
        for e in sol.ends(b.regions.universal_vars[u], universals) {
            if b.regions.known[u][e] {
                continue;
            }
            if propagate && u < signature && e < signature {
                reqs.regions.push((u, e));
                continue;
            }
            let path = b.regions.blame(b.regions.universal_vars[u], e, &sol);
            let span = path.last().map_or(Span::new(0, 0), |c| c.span);
            let (long, short) = (names.lifetime(b, u), names.lifetime(b, e));
            // In a closure's body, something it got through itself must
            // last longer than the call.
            if b.regions.call == Some(u) {
                let gives = match b.body.locals[0]
                    .ty
                    .regions()
                    .iter()
                    .any(|r| **r == Region::Var(b.regions.universal_vars[e]))
                {
                    true => "returns",
                    false => "gives out",
                };
                errors.push(Error::ty(
                    span,
                    format!(
                        "this closure {gives} a reference to what it captured, which only lasts \
                         as long as the call: the next call could change it, or the closure \
                         could be gone"
                    ),
                ));
                continue;
            }
            if let Some(error) = closure_generality(b, names, u, e, span) {
                errors.push(error);
                continue;
            }
            let general = "a function type here takes references of any lifetime, as its \
                           caller picks";
            let message = match (u >= signature, e >= signature) {
                (false, false) => format!(
                    "lifetime may not live long enough: this needs {long} to outlive {short}, \
                     and the signature doesn't say it does"
                ),
                (true, false) => format!(
                    "this function isn't general enough: {general}, but this needs the \
                     lifetime to outlive {short}"
                ),
                (false, true) => format!(
                    "this function isn't general enough: {general}, but this needs {long} to \
                     outlive it"
                ),
                (true, true) => format!(
                    "this function isn't general enough: {general}, but this needs one of \
                     them to outlive another"
                ),
            };
            let mut err = Error::ty(span, message);
            if let (Region::Named(l), Region::Named(s)) =
                (&b.regions.universal[u], &b.regions.universal[e])
                && u < signature
                && e < signature
                && !b.regions.universal[u].is_elided()
                && !b.regions.universal[e].is_elided()
            {
                err = err.with_note(
                    names.generics.unwrap_or(span),
                    format!("if it does, say so with a bound: `{l}: {s}`"),
                );
            }
            errors.push(err);
        }
    }
    for (p, r, span) in b.regions.type_tests.clone() {
        let known = b.regions.known_of(&p);
        for e in sol.ends(r, universals) {
            if known.contains(&e) {
                continue;
            }
            if propagate && e < signature {
                reqs.params.push((p.clone(), e));
                continue;
            }
            let short = names.lifetime(b, e);
            // A projection outlives what its type does.
            let (what, p) = match &p {
                Type::Proj(..) => ("type", root_param(&p)),
                p => ("type parameter", p.clone()),
            };
            let bound = match &b.regions.universal[e] {
                Region::Named(n) if !b.regions.universal[e].is_elided() && e < signature => {
                    format!("`{p}: {n}`")
                }
                Region::Static => format!("`{p}: 'static`"),
                _ => format!("like `{p}: 'a`, naming the lifetime `'a`"),
            };
            errors.push(Error::ty(
                span,
                format!(
                    "the {what} `{p}` may not live long enough: this needs it to outlive \
                     {short}; say it does with a bound, {bound}"
                ),
            ));
        }
    }
    if let Some(e) = errors.into_iter().min_by_key(|e| e.span.start).or(members) {
        return Err(e);
    }
    for (name, span) in moves::unused_mut(b, &needed) {
        warnings.push(Warning {
            lint: Lint::UnusedMut,
            span,
            message: format!("variable `{name}` doesn't need to be mutable"),
            fixes: vec![Fix::new("remove `mut`", vec![Edit::delete(span)])],
        });
    }
    Ok((warnings, reqs))
}

/// What's wrong, if universal region `u` must outlive `e` in a closure's
/// body, at `span`, and one of them is a lifetime the closure is generic
/// over: each call picks it, so it could be anything.
fn closure_generality(b: &Builder, names: &Names, u: usize, e: usize, span: Span) -> Option<Error> {
    let (long, short) = (names.lifetime(b, u), names.lifetime(b, e));
    let message = match (names.bound_param(u), names.bound_param(e)) {
        (None, None) => return None,
        (Some(p), None) if names.ret.contains(&b.regions.universal[e]) => {
            format!(
                "this closure returns a reference from `{p}`, whose lifetime each call picks, \
                 but its result doesn't have that lifetime"
            )
        }
        (Some(p), None) => match names.capture(b, e) {
            Some(c) => format!(
                "this closure stores a reference from `{p}` in `{c}`, but each call picks how \
                 long `{p}` lives, and the next could give it one that's gone first"
            ),
            None => format!(
                "this closure keeps a reference from `{p}` for {short}, but each call picks how \
                 long `{p}` lives, which could be shorter"
            ),
        },
        (None, Some(p))
            if b.body.locals[0]
                .ty
                .regions()
                .contains(&&Region::Var(b.regions.universal_vars[e])) =>
        {
            let lives = match names.capture(b, u) {
                Some(c) => format!("this only lives as long as the captured `{c}`"),
                None => format!("this needs {long} to outlive it"),
            };
            format!(
                "this closure's result has the lifetime of `{p}`, which each call picks, so it \
                 could be any, even `'static`, but {lives}"
            )
        }
        (None, Some(_)) => format!(
            "this needs {long} to outlive {short}, which each call picks, so it could be any, \
             even `'static`"
        ),
        (Some(_), Some(_)) => format!(
            "this needs {long} to outlive {short}, but each call picks those, so they could be \
             any"
        ),
    };
    Some(Error::ty(span, message))
}

/// The type parameter at the bottom of rigid projection `t`: `T` for
/// `<T::Out as Tr>::Item`.
fn root_param(t: &Type) -> Type {
    match t {
        Type::Proj(inner, ..) => root_param(inner),
        t => t.clone(),
    }
}

/// Checks closure `c`, found in `parent`'s body, as a function of its own.
///
/// The universal regions of the function it's in are universal in it too,
/// as what they are there: what it needs of them goes up as it is. Then
/// each region in its signature, and for one that captures, in its
/// fields' types, is a lifetime parameter of its own, which it needs
/// what it needs of, for its type to say, as constraints at the closure
/// (see `analyze`).
///
/// But for the lifetimes its signature is generic over (`FnType::bound`),
/// which each call picks: each is a placeholder, after the signature's
/// universal regions, so the body can assume nothing of it, and anything it
/// needs of it is an error here, not a constraint that goes up, as the
/// function it's in can't know what a call will pick. That's what makes
/// any instantiation at a call sound (docs/06e.1-higher-ranked-closures.md).
/// A placeholder has no implied bounds. Its result's lifetimes were worked
/// out from its body already (`decide_result`), so they're checked here
/// like any written out.
///
/// One that captures is called with itself for its first parameter, by
/// `&` or `&mut`, as it's `Fn` or `FnMut`, for a lifetime only that call
/// knows, which nothing it gives out may need, as the next call may
/// change what it points to; or by value, as it's `FnOnce`. What it
/// captured are that parameter's fields.
fn closure(parent: &Builder, c: &ClosureUse) -> Result<(Vec<Warning>, Requirements), Error> {
    let external: Vec<String> = parent.regions.universal[1..parent.regions.signature]
        .iter()
        .map(|r| match r {
            Region::Named(n) => n.clone(),
            r => unreachable!("universal region `{r}`"),
        })
        .collect();
    match closure_body(parent.cx, parent.info, external, c, None)? {
        Outcome::Checked(warnings, reqs) => Ok((warnings, reqs)),
        Outcome::Decided(_) => unreachable!("checking, not deciding"),
    }
}

/// What the lifetimes of the result of closure `lambda`, of type `ty`,
/// named `later`, are, worked out from its body once the function it's in
/// has its types: each the one lifetime the closure is generic over that
/// flows into it, if one does, which the result then borrows from, or if
/// none does, one fixed where the closure is made (`Region::Erased`), such
/// as of what it captured, if anything flows into it. If nothing does, as
/// for `None`, it's as for a function: the lifetime the closure is generic
/// over, if there's one. If two do, the result would have to live as long
/// as both, which its type can't say. The borrow checker then checks the
/// body against that (`closure`), so this only picks the signature: any
/// choice is as sound as any other.
pub(crate) fn decide_result(
    cx: &Checker,
    lambda: &Lambda,
    ty: &Type,
    later: usize,
) -> Result<Vec<Region>, Error> {
    let info = AdtInfo::new(cx.adts());
    let c = ClosureUse {
        lambda,
        ty: ty.clone(),
        span: lambda.body.span,
    };
    match closure_body(cx, &info, Vec::new(), &c, Some(later))? {
        Outcome::Decided(regions) => Ok(regions),
        Outcome::Checked(..) => unreachable!("deciding, not checking"),
    }
}

/// What `closure_body` found: what checking a closure's body gave, or the
/// lifetimes of its result it worked out.
enum Outcome {
    Checked(Vec<Warning>, Requirements),
    Decided(Vec<Region>),
}

/// Checks closure `c` as a function of its own, whose external lifetimes
/// are `external` (see `closure`), or, if its result's lifetimes named
/// `later` are to be worked out, works them out (see `decide_result`).
fn closure_body(
    cx: &Checker,
    info: &AdtInfo,
    external: Vec<String>,
    c: &ClosureUse,
    later: Option<usize>,
) -> Result<Outcome, Error> {
    let mut lifetimes = external.clone();
    let typed = cx.typed();
    let mut captures = Vec::new();
    let (sig, env) = match &c.ty {
        Type::Fn(sig) => {
            let sig = Type::Fn(sig.clone()).map_regions(&mut |r| {
                if is_result_lifetime(r) {
                    return r.clone();
                }
                let name = format!("'_c{}", lifetimes.len());
                lifetimes.push(name.clone());
                Region::Named(name)
            });
            (sig, None)
        }
        Type::Adt(symbol, _, args) => {
            let def = &cx.traits().closures[symbol];
            lifetimes.extend(def.lifetimes.iter().cloned());
            captures = def
                .fields
                .iter()
                .map(|(n, t)| (n.clone(), t.regions().into_iter().cloned().collect()))
                .collect();
            let own = Type::Adt(
                symbol.clone(),
                def.lifetimes.iter().cloned().map(Region::Named).collect(),
                args.clone(),
            );
            let modes = &typed.modes[&c.lambda.id];
            let vars = modes
                .iter()
                .map(|(id, capture)| {
                    let bind = &typed.binds[id];
                    EnvVar {
                        name: bind.name.clone(),
                        capture: *capture,
                        id: *id,
                        span: bind.span,
                        mutable: bind.mutable,
                    }
                })
                .collect();
            let by_ref = match def.kind.expect("settled") {
                FnKind::Fn => Some(false),
                FnKind::FnMut => Some(true),
                FnKind::FnOnce => None,
            };
            (def.sig_at(&own), Some((own, by_ref, vars)))
        }
        t => unreachable!("a closure of type `{t}`"),
    };
    let Type::Fn(sig) = &sig else {
        unreachable!("a closure's signature is a function type")
    };
    // What the function knows of them, it checks of what goes up.
    let mut regions = Regions::new(&lifetimes, &[]);
    // The lifetimes it's generic over are placeholders: each call picks
    // them, so the body can assume nothing of them, and needs nothing of
    // them that could go up, as the function it's in can't see to that.
    let bound: Vec<Region> = sig
        .bound
        .iter()
        .map(|_| Region::Var(regions.placeholder()))
        .collect();
    let (params, ret) = sig.instantiate(&bound);
    // Each lifetime of its result to work out is a variable, of the body's.
    let later = later.unwrap_or(0);
    let mut vars: Vec<Option<usize>> = vec![None; later];
    let ret = ret.map_regions(&mut |r| match r {
        Region::Result(l, i) if *l == c.lambda.id && *i < later => {
            Region::Var(*vars[*i].get_or_insert_with(|| regions.fresh()))
        }
        r => r.clone(),
    });
    let first = regions.universal.len() - bound.len();
    let names = Names {
        params: Vec::new(),
        generics: None,
        bound: (first..regions.universal.len())
            .zip(&bound)
            .map(|(u, r)| {
                let param = c
                    .lambda
                    .params
                    .iter()
                    .zip(&params)
                    .find(|(_, t)| t.regions().contains(&r));
                (u, param.map_or("_".into(), |(p, _)| p.name.name.clone()))
            })
            .collect(),
        captures,
        ret: ret
            .regions()
            .into_iter()
            .filter(|r| {
                matches!(r, Region::Named(n) if !external.contains(n)) || is_result_lifetime(r)
            })
            .cloned()
            .collect(),
    };
    let adts = local_adts(cx, &regions);
    let env = env.map(|(own, by_ref, vars)| {
        let own = regions.renumber(&own);
        let t = match by_ref {
            Some(m) => Type::Ref(Region::Var(regions.call_region()), m, Box::new(own)),
            None => own,
        };
        (t, by_ref, vars, c.span)
    });
    let mut b = Builder::new(cx, &adts, info, regions);
    b.closure = true;
    let ret = b.regions.renumber(&ret);
    let params: Vec<(String, NodeId, Type)> = c
        .lambda
        .params
        .iter()
        .zip(&params)
        .map(|(p, t)| (p.name.name.clone(), p.name.id, b.regions.renumber(t)))
        .collect();
    b.signature(ret, &params, env);
    b.lower(&c.lambda.body);
    if later == 0 {
        let (warnings, reqs) = analyze(&mut b, &names, true)?;
        return Ok(Outcome::Checked(warnings, reqs));
    }
    closures_in(&mut b);
    let bound: Vec<usize> = bound.iter().map(|r| b.regions.universal_var(r)).collect();
    let decided = vars
        .iter()
        .map(|v| match v {
            Some(v) => decide(&b, &names, &sig.bound, &bound, *v, c),
            // Not in its type any more: nothing to decide.
            None => Ok(Region::Erased),
        })
        .collect::<Result<_, _>>()?;
    Ok(Outcome::Decided(decided))
}

/// What region variable `v`, a lifetime of the result of closure `c`, whose
/// body is lowered in `b`, is: one of the lifetimes it's generic over,
/// `bound`, whose variables are `vars`, or one fixed where it's made (see
/// `decide_result`).
fn decide(
    b: &Builder,
    names: &Names,
    bound: &[String],
    vars: &[usize],
    v: usize,
    c: &ClosureUse,
) -> Result<Region, Error> {
    // Everything that must outlive `v`: what flows into it.
    let mut outliving: HashMap<usize, Vec<usize>> = HashMap::new();
    for o in &b.regions.constraints {
        outliving.entry(o.sub).or_default().push(o.sup);
    }
    let mut seen = HashSet::from([v]);
    let mut todo = vec![v];
    while let Some(r) = todo.pop() {
        for s in outliving.get(&r).into_iter().flatten() {
            if seen.insert(*s) {
                todo.push(*s);
            }
        }
    }
    let from: Vec<usize> = (0..vars.len())
        .filter(|i| seen.contains(&vars[*i]))
        .collect();
    // A borrow, or a universal region but `'static` and those it's generic
    // over: what it captured, or borrows of what's in it.
    let fixed = seen.iter().any(|r| {
        b.loans.iter().any(|l| l.region == *r)
            || (b.regions.universal_vars[1..].contains(r) && !vars.contains(r))
    });
    match from[..] {
        [i] => Ok(Region::Named(bound[i].clone())),
        [] if fixed || bound.len() != 1 => Ok(Region::Erased),
        [] => Ok(Region::Named(bound[0].clone())),
        [i, j, ..] => {
            let name = |k: usize| names.bound.get(k).map_or("_", |(_, p)| p.as_str());
            Err(Error::ty(
                result_span(&c.lambda.body),
                format!(
                    "this closure's result could borrow from `{}` or from `{}`, and each call \
                     picks how long each of those lives, apart: its type can't say the result \
                     lives as long as both",
                    name(i),
                    name(j)
                ),
            ))
        }
    }
}

/// For naming lifetimes in messages: which parameter each elided one is
/// in, and where the generic parameters are, to suggest bounds there. For a
/// closure, which parameter each lifetime it's generic over is in, by its
/// universal region, and the regions of each capture and of its result.
struct Names {
    params: Vec<(String, Vec<Region>)>,
    generics: Option<Span>,
    bound: Vec<(usize, String)>,
    captures: Vec<(String, Vec<Region>)>,
    ret: Vec<Region>,
}

impl Names {
    fn of(f: &FnDecl, sig: &FnType) -> Names {
        Names {
            params: f
                .params
                .iter()
                .zip(&sig.params)
                .map(|(p, t)| {
                    (
                        p.name.name.clone(),
                        t.regions().into_iter().cloned().collect(),
                    )
                })
                .collect(),
            generics: f.generics.span,
            bound: Vec::new(),
            captures: Vec::new(),
            ret: Vec::new(),
        }
    }

    /// The parameter universal region `u` is in, if it's one of those the
    /// closure being checked is generic over.
    fn bound_param(&self, u: usize) -> Option<&str> {
        self.bound
            .iter()
            .find(|(b, _)| *b == u)
            .map(|(_, p)| p.as_str())
    }

    /// The capture whose type universal region `u` is in, if it's one.
    fn capture(&self, b: &Builder, u: usize) -> Option<&str> {
        let r = &b.regions.universal[u];
        self.captures
            .iter()
            .find(|(_, rs)| rs.contains(r))
            .map(|(n, _)| n.as_str())
    }

    /// How to name universal region `u` of `b` in a message.
    fn lifetime(&self, b: &Builder, u: usize) -> String {
        let r = &b.regions.universal[u];
        if let Some(p) = self.bound_param(u) {
            return format!("the lifetime of `{p}`");
        }
        if u >= b.regions.signature {
            return "a lifetime a call picks, which can be anything".to_string();
        }
        if let Some(c) = self.capture(b, u) {
            return format!("the lifetime of the captured `{c}`");
        }
        match r {
            Region::Static => "`'static`".to_string(),
            Region::Named(n) if !r.is_elided() => format!("`{n}`"),
            Region::Result(..) => format!("`{r}`"),
            _ => match self.params.iter().find(|(_, rs)| rs.contains(r)) {
                Some((p, _)) => format!("the lifetime of `{p}`"),
                None => "the closure's lifetime".to_string(),
            },
        }
    }
}

/// How to write `place` in a message: `x`, `p.x`, `*r`.
fn describe(b: &Builder, place: &Place) -> String {
    let decl = &b.body.locals[place.local.0];
    let mut out = match (&decl.name, decl.kind) {
        (Some(n), _) => n.clone(),
        (None, LocalKind::Return) => "the result".to_string(),
        (None, LocalKind::Env) => "the closure".to_string(),
        (None, _) => "a temporary".to_string(),
    };
    let mut t = decl.ty.clone();
    let mut proj = &place.proj[..];
    // In a closure's body, what it captured is written as the variable:
    // `x` for `*(*env).x`, where it has a reference to `x`.
    if let Some(v) = moves::env_var(b, place) {
        out = v.name.clone();
        let mut skip = usize::from(matches!(t, Type::Ref(..))) + 1;
        if v.capture != crate::ast::Capture::Value {
            skip += 1;
        }
        for p in proj.iter().take(skip) {
            t = build::project(b.adts(), &t, *p);
        }
        proj = &proj[skip.min(proj.len())..];
    }
    for (i, p) in proj.iter().enumerate() {
        match p {
            // `(*r).x` is written `r.x`.
            Proj::Deref if matches!(proj.get(i + 1), Some(Proj::Field(_))) => {}
            Proj::Deref => out = format!("*{out}"),
            Proj::Field(f) => {
                let fields = crate::types::struct_fields(b.adts(), &t).unwrap_or_default();
                let name = fields.get(*f).map_or(f.to_string(), |(n, _)| n.clone());
                out = format!("{out}.{name}");
            }
            Proj::Variant(_, f) => out = format!("{out}.{f}"),
        }
        t = build::project(b.adts(), &t, *p);
    }
    out
}
