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
use crate::check::Checker;
use crate::error::{Edit, Error, Fix, Lint, Span, Warning};
use crate::types::{FnType, Region, Scheme, Type};
use build::{Builder, ClosureUse};
use mir::{LocalKind, Place, Proj};
use regions::{AdtInfo, Regions, wf_bounds};

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
    let mut b = Builder::new(cx, &info, Regions::new(&scheme.lifetimes, &bounds));
    let ret = b.regions.renumber(&sig.ret);
    let params: Vec<(String, Span, Type)> = f
        .params
        .iter()
        .zip(&sig.params)
        .map(|(p, t)| (p.name.name.clone(), p.name.span, b.regions.renumber(t)))
        .collect();
    b.signature(ret, &params, &[]);
    b.lower(&f.body);
    let names = Names::of(f, sig);
    let (warnings, _) = analyze(&mut b, &names, false)?;
    Ok(warnings)
}

/// What a closure needs of the regions in its signature, which the
/// function it's in has to see to: that universal region `.0` outlives
/// universal `.1`, or that type parameter `.0` outlives universal `.1`.
#[derive(Default)]
struct Requirements {
    regions: Vec<(usize, usize)>,
    params: Vec<(String, usize)>,
}

/// Checks a lowered body, its closures first. For a closure, `propagate`:
/// what it needs of its universal regions is returned rather than an error.
fn analyze(
    b: &mut Builder,
    names: &Names,
    propagate: bool,
) -> Result<(Vec<Warning>, Requirements), Error> {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    for c in std::mem::take(&mut b.closures) {
        match closure(b, &c) {
            Ok((w, reqs)) => {
                warnings.extend(w);
                let regions = c.ty.regions().into_iter().cloned().collect::<Vec<_>>();
                let region = |u: usize| match u {
                    0 => Region::Static,
                    u => regions[u - 1].clone(),
                };
                for (u, e) in reqs.regions {
                    b.regions.outlives(&region(u), &region(e), c.span);
                }
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
    let needed = match moves::behind_references(b).and_then(|()| moves::mutability(b)) {
        Ok(needed) => needed,
        Err(e) => {
            errors.push(e);
            Vec::new()
        }
    };
    if let Err(e) = moves::moves(b) {
        errors.push(e);
    }
    let sol = b.regions.solve(&b.body);
    if let Err(e) = loans::check(b, &sol) {
        errors.push(e);
    }
    let mut reqs = Requirements::default();
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
        let known = b.regions.known_params.get(&p).cloned().unwrap_or_default();
        for e in sol.ends(r, universals) {
            if known.contains(&e) {
                continue;
            }
            if propagate && e < signature {
                reqs.params.push((p.clone(), e));
                continue;
            }
            let short = names.lifetime(b, e);
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
                    "the type parameter `{p}` may not live long enough: this needs it to outlive \
                     {short}; say it does with a bound, {bound}"
                ),
            ));
        }
    }
    if let Some(e) = errors.into_iter().min_by_key(|e| e.span.start) {
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

/// Checks closure `c`, found in `parent`'s body, as a function of its own.
fn closure(parent: &Builder, c: &ClosureUse) -> Result<(Vec<Warning>, Requirements), Error> {
    let cx = parent.cx;
    let Type::Fn(sig) = &c.ty else {
        unreachable!("a closure has a function type")
    };
    // Each region in its signature is a lifetime parameter of its own.
    let mut lifetimes = Vec::new();
    let sig = Type::Fn(sig.clone()).map_regions(&mut |_| {
        let name = format!("'_c{}", lifetimes.len());
        lifetimes.push(name.clone());
        Region::Named(name)
    });
    let Type::Fn(FnType { params, ret, .. }) = &sig else {
        unreachable!()
    };
    let mut b = Builder::new(cx, parent.info, Regions::new(&lifetimes, &[]));
    b.closure = true;
    let ret = b.regions.renumber(ret);
    let params: Vec<(String, Span, Type)> = c
        .lambda
        .params
        .iter()
        .zip(params)
        .map(|(p, t)| (p.name.name.clone(), p.name.span, b.regions.renumber(t)))
        .collect();
    let typed = cx.typed();
    let captured: Vec<(String, Span, Type)> = typed.captures[&std::ptr::from_ref(c.lambda)]
        .iter()
        .map(|id| {
            let bind = &typed.binds[id];
            let t = cx.solution(&bind.ty);
            (bind.name.clone(), *id, b.regions.fresh_regions(&t))
        })
        .collect();
    b.signature(ret, &params, &captured);
    b.lower(&c.lambda.body);
    let names = Names {
        params: Vec::new(),
        generics: None,
    };
    analyze(&mut b, &names, true)
}

/// For naming lifetimes in messages: which parameter each elided one is
/// in, and where the generic parameters are, to suggest bounds there.
struct Names {
    params: Vec<(String, Vec<Region>)>,
    generics: Option<Span>,
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
        }
    }

    /// How to name universal region `u` of `b` in a message.
    fn lifetime(&self, b: &Builder, u: usize) -> String {
        let r = &b.regions.universal[u];
        if u >= b.regions.signature {
            return "a lifetime a call picks, which can be anything".to_string();
        }
        match r {
            Region::Static => "`'static`".to_string(),
            Region::Named(n) if !r.is_elided() => format!("`{n}`"),
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
        (None, _) => "a temporary".to_string(),
    };
    let mut t = decl.ty.clone();
    for (i, p) in place.proj.iter().enumerate() {
        match p {
            // `(*r).x` is written `r.x`.
            Proj::Deref if matches!(place.proj.get(i + 1), Some(Proj::Field(_))) => {}
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
