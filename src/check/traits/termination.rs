//! Rules that make trait resolution and normalising projections end
//! (*termination*), and keep associated types finite.

use super::{each_proj, fields, has_proj, mentions, occurrences};
use crate::check::Checker;
use crate::error::{Error, Span};
use crate::parser::is_effect_name;
use crate::traits::ImplDef;
use crate::types::{Subst, Type};
use std::collections::HashSet;

impl Checker {
    /// Proving a bound of an impl on what isn't a part of its type, like
    /// `I::Item: Discard` in `impl<I: Iterator> Iterator for Filter<I, P>
    /// where I::Item: Discard`, or on a parameter a bound decides, is about
    /// a type that can be bigger than the one the impl is for: with
    /// `impl<I: Iterator> A for W<I> where I::Item: B` and `impl<T: A> B
    /// for V<T>`, an `X` whose `Item` is `V<W<X>>` has `W<X>: A` need
    /// `V<W<X>>: B`, which needs `W<X>: A` again. So such a bound, in an
    /// impl of `A`, can only be of a trait that can't lead back to `A`, by
    /// any impl's bounds.
    ///
    /// Then the search ends. Take the traits' graph, with an edge from
    /// each trait to those its impls' bounds are of, and its strongly
    /// connected components: each step of a proof stays in one, and is a
    /// bound on a part of the impl's type, or a blanket impl's (which
    /// `termination` sees to), or goes to a component lower down, which
    /// can only happen as often as there are components.
    ///
    /// The packages before the one being checked kept this on their own,
    /// so a way back that's new goes through one of its impls, which is
    /// what's at fault.
    pub(super) fn bound_termination(&self) -> Result<(), Error> {
        let mut edges: Vec<(&str, &str, &ImplDef)> = Vec::new();
        for d in &self.traits.impls {
            let Some(tr) = &d.trait_name else { continue };
            for pred in &d.preds {
                edges.push((tr, &pred.trait_name, d));
            }
        }
        for d in self.newest_first() {
            let Some(tr) = &d.trait_name else { continue };
            let part = |t: &Type| matches!(t, Type::Param(p) if mentions(&d.self_ty, p));
            for pred in d.preds.iter().filter(|p| !part(&p.ty)) {
                let mut seen = HashSet::new();
                let to = pred.trait_name.as_str();
                let Some(back) = reach(&edges, to, tr, &mut seen) else {
                    continue;
                };
                if !self.current(d.span)
                    && let Some(own) = back.iter().find(|b| self.current(b.span))
                {
                    return Err(Error::ty(
                        own.span,
                        format!(
                            "proving that a type implements `{tr}` could go on forever: `impl \
                             {}` needs `{}: {to}`, which isn't a part of `{}`, so it can be a \
                             bigger type, and this impl lets proving that lead back to `{tr}`",
                            d.header(),
                            pred.ty,
                            d.self_ty
                        ),
                    ));
                }
                let how = match back.first() {
                    None => String::new(),
                    Some(b) => format!(", by the bounds of `impl {}`", b.header()),
                };
                return Err(Error::ty(
                    d.span,
                    format!(
                        "proving that a type implements `{tr}` could go on forever: this impl \
                         needs `{}: {to}`, which isn't a part of `{}`, so it can be a bigger \
                         type, and proving it could lead back to `{tr}`{how}",
                        pred.ty, d.self_ty
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Working out a projection ends (see `Projections`): each step, by an
    /// impl, is to projections on smaller types (checked where each impl's
    /// associated types are, in `terminates`), or, by a blanket impl `impl<T>
    /// B for T`, on `T` itself, to another trait's. Those steps are edges
    /// between traits, and like the blanket impls' bounds (`termination`)
    /// can't go round in a cycle. Then each step goes down in size, or stays
    /// the same size and goes down in an order of the traits, and that
    /// can't go on forever.
    ///
    /// A new cycle has an edge of the package being checked, which is
    /// what's reported: those are looked at first.
    pub(super) fn assoc_termination(&self) -> Result<(), Error> {
        let mut edges: Vec<(&str, &str, &ImplDef)> = Vec::new();
        for d in &self.traits.impls {
            let (Some(tr), Type::Param(_)) = (&d.trait_name, &d.self_ty) else {
                continue;
            };
            // What a bound decides a parameter is, is worked out when the
            // impl's associated types are (`traits::impl_args`).
            let deciding = d.params.iter().filter_map(|p| {
                let decided = !is_effect_name(p) && !mentions(&d.self_ty, p);
                let k = crate::traits::decider(&d.self_ty, &d.params, &d.eqs, p);
                decided.then_some(&d.eqs[k?].0)
            });
            for t in d.assoc.iter().map(|(_, t)| t).chain(deciding) {
                each_proj(t, &mut |s, to| {
                    if *s == d.self_ty {
                        edges.push((tr, to, d));
                    }
                });
            }
        }
        for &(from, to, d) in self.newest_edges(&edges) {
            let mut seen = HashSet::new();
            if reach(&edges, to, from, &mut seen).is_some() {
                return Err(Error::ty(
                    d.span,
                    format!(
                        "working out an associated type of `{from}` could go on forever: \
                         blanket impls work out `{from}`'s from `{to}`'s and `{to}`'s from \
                         `{from}`'s, for the same type, which isn't smaller each time"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// No type holds itself by value through a projection: `struct S<T:
    /// Tr> { x: T::Out }` with `impl Tr for i64 { type Out = S<i64>; }` has
    /// an `S<i64>` hold an `S<i64>`, so it would have infinite size, which
    /// neither says alone (`collect_adts` checks each struct and enum on
    /// its own). Every such cycle goes through some impl's associated type,
    /// so from each, what it holds by value is followed, through fields and
    /// the impls projections are worked out by: getting back to an impl
    /// already on the way is a type that holds itself, or holds a bigger
    /// one of itself, again and again, which is as infinite.
    ///
    /// Only the package being checked's impls are looked at: an earlier
    /// package's associated types can't get to its impls, as they can only
    /// project on their own types and type parameters, and a projection on
    /// a parameter is worked out by no impl.
    pub(super) fn assoc_sizes(&self) -> Result<(), Error> {
        let own = self.traits.impls.iter().enumerate().skip(self.impls_from);
        for (k, d) in own {
            for (name, t) in &d.assoc {
                let mut path = vec![k];
                if let Some(back) = self.holds(t, &mut path, &mut HashSet::new()) {
                    let tr = d.trait_name.as_deref().unwrap_or_default();
                    return Err(Error::ty(
                        d.span,
                        format!(
                            "`{t}`, which this impl says `<{} as {tr}>::{name}` is, contains \
                             itself through `{back}`, so it would have infinite size; hold it \
                             through a pointer, like `Box<{t}>`, instead",
                            d.self_ty
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The projection by which `t` holds by value what an impl on `path`
    /// says an associated type is, if it does.
    fn holds(&self, t: &Type, path: &mut Vec<usize>, done: &mut HashSet<Type>) -> Option<Type> {
        match t {
            Type::Adt(name, regions, args) => {
                if !done.insert(t.erased()) {
                    return None;
                }
                let def = &self.adts[name];
                let s = Subst::new(&def.params, args).with_regions(&def.lifetimes, regions);
                fields(&def.kind)
                    .iter()
                    .find_map(|(_, f)| self.holds(&s.apply(f), path, done))
            }
            Type::Proj(s, tr, name) => {
                let (k, _) = self.traits.select(s, Some(tr), &[])?;
                if path.contains(&k) {
                    return Some(t.clone());
                }
                let (impls, anon) = (&self.traits.impls, &self.traits.anon());
                let u = crate::traits::project(impls, anon, &[], s, tr, name)?;
                path.push(k);
                let found = self.holds(&u, path, &mut HashSet::new());
                path.pop();
                found
            }
            _ => None,
        }
    }

    /// Proving `T: A` by an impl needs what the impl needs. Those on its
    /// type parameters that are parts of the type it's for are about
    /// smaller types, and the search ends: the *Paterson conditions*,
    /// which a bound on a type parameter meets whenever the impl is for a
    /// type that's more than that parameter. The others, on projections
    /// and on parameters a bound decides, are `bound_termination`'s.
    ///
    /// An impl for a bare parameter, a *blanket impl* like `impl<T: A> B
    /// for T`, needs `T: A` to prove `T: B`: the same type. That still ends
    /// unless there's a way back from `A` to `B`, like `impl<T: B> A for T`,
    /// so the blanket impls between traits can't form a cycle.
    ///
    /// A new cycle has an edge of the package being checked, which is
    /// what's reported: those are looked at first.
    pub(super) fn termination(&self) -> Result<(), Error> {
        // An edge from each trait a blanket impl is of to each it needs.
        let mut edges: Vec<(&str, &str, &ImplDef)> = Vec::new();
        for d in &self.traits.impls {
            let (Some(tr), Type::Param(p)) = (&d.trait_name, &d.self_ty) else {
                continue;
            };
            for pred in d.preds.iter().filter(|q| q.ty == Type::Param(p.clone())) {
                edges.push((tr, &pred.trait_name, d));
            }
        }
        for &(from, to, d) in self.newest_edges(&edges) {
            // A path back from `to` to `from`.
            let mut path: Vec<&ImplDef> = vec![d];
            let mut seen = HashSet::new();
            if let Some(back) = reach(&edges, to, from, &mut seen) {
                path.extend(back);
                let impls: Vec<String> = path
                    .iter()
                    .map(|d| {
                        let bounds: Vec<String> =
                            d.preds.iter().map(|p| p.trait_name.clone()).collect();
                        format!(
                            "`impl<{}: {}> {}`",
                            d.self_ty,
                            bounds.join(" + "),
                            d.header()
                        )
                    })
                    .collect();
                let message = match impls.len() {
                    1 => format!(
                        "proving that a type implements `{from}` would go on forever: {} needs \
                         it to prove it",
                        impls[0]
                    ),
                    _ => format!(
                        "proving that a type implements `{from}` could go on forever: {} need \
                         each other, for the same type",
                        impls.join(" and ")
                    ),
                };
                return Err(Error::ty(d.span, message));
            }
        }
        Ok(())
    }
}

impl Checker {
    /// Every impl, the package being checked's first.
    fn newest_first(&self) -> impl Iterator<Item = &ImplDef> {
        let impls = &self.traits.impls;
        impls[self.impls_from..]
            .iter()
            .chain(&impls[..self.impls_from])
    }

    /// `edges`, those of impls of the package being checked first.
    fn newest_edges<'e, 'a>(
        &self,
        edges: &'e [(&'a str, &'a str, &'a ImplDef)],
    ) -> impl Iterator<Item = &'e (&'a str, &'a str, &'a ImplDef)> {
        let (own, older): (Vec<_>, Vec<_>) = edges.iter().partition(|e| self.current(e.2.span));
        own.into_iter().chain(older)
    }
}

/// A path through `edges` from `from` to `to`, as the impls on it.
fn reach<'a>(
    edges: &[(&'a str, &'a str, &'a ImplDef)],
    from: &'a str,
    to: &'a str,
    seen: &mut HashSet<&'a str>,
) -> Option<Vec<&'a ImplDef>> {
    if from == to {
        return Some(Vec::new());
    }
    if !seen.insert(from) {
        return None;
    }
    for &(a, b, d) in edges {
        if a == from
            && let Some(mut rest) = reach(edges, b, to, seen)
        {
            rest.insert(0, d);
            return Some(rest);
        }
    }
    None
}

/// How many type constructors `t` is made of.
fn size(t: &Type) -> usize {
    1 + t.parts().map(size).sum::<usize>()
}

/// Checks that working out the projections in associated type `t`,
/// written at `span` in an impl for `self_ty` with type parameters
/// `params`, ends: each is on a smaller type than `self_ty`, with no
/// parameter more often, and no projection inside (GHC's conditions for
/// type families). In a blanket impl, one on its parameter itself is fine
/// too: `assoc_termination` sees to that.
pub(super) fn terminates(
    t: &Type,
    self_ty: &Type,
    params: &[String],
    span: Span,
) -> Result<(), Error> {
    if let Type::Proj(s, tr, name) = t {
        let blanket = matches!(self_ty, Type::Param(_)) && **s == *self_ty;
        let ok = blanket
            || (!has_proj(s)
                && size(s) < size(self_ty)
                && params
                    .iter()
                    .all(|p| occurrences(s, p) <= occurrences(self_ty, p)));
        if !ok {
            return Err(Error::ty(
                span,
                format!(
                    "working out `<{s} as {tr}>::{name}` here might never end: it must be on \
                     a smaller type than `{self_ty}`, the type the impl is for, built from its \
                     parts, so that each step is about a smaller type"
                ),
            ));
        }
        return Ok(());
    }
    t.parts()
        .try_for_each(|p| terminates(p, self_ty, params, span))
}
