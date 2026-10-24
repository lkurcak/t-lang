//! Bounds and `where` clauses: the predicates and equality bounds they
//! say, and the projections in them.

use crate::ast::*;
use crate::check::Checker;
use crate::check::generics::Lifetimes;
use crate::error::Error;
use crate::traits::FN_ONCE;
use crate::traits::Pred;
use crate::types::Type;

impl Checker {
    /// The trait bounds in `g`, whose type parameters are `params` and
    /// lifetimes `lifetimes`, and what its equality bounds say: `T: Iter`
    /// and `(T::Item, i64)` for `T: Iter<Item = i64>`. Bounds on
    /// projections, like `T::Item: Discard` for `T: Iter<Item: Discard>`,
    /// only if `on_projections`: a function's, not an impl's or a struct's,
    /// whose bounds resolution would then have to prove of what `T::Item`
    /// stands for, which may be a bigger type than the impl's.
    pub(in crate::check) fn preds(
        &mut self,
        g: &Generics,
        params: &crate::types::Params,
        lifetimes: &[String],
        on_projections: bool,
    ) -> Result<(Vec<Pred>, Vec<Equality>), Error> {
        // A type in a bound can name an associated type of a parameter,
        // as `F: Fn(I::Item)` does, which the parameter's own bounds say
        // which trait's it is: in scope while the types are worked out.
        let saved = self.env.clone();
        for (p, after) in &g.bounds {
            for b in after.iter().filter(|b| !b.name.name.starts_with('\'')) {
                if params.contains(&p.name)
                    && let Ok(tr) = self.trait_named(&b.name)
                {
                    self.env.push(Pred {
                        ty: Type::Param(p.name.clone()),
                        trait_name: tr,
                    });
                }
            }
        }
        let result = self.preds_in(g, params, lifetimes, on_projections);
        self.env = saved;
        result
    }

    fn preds_in(
        &mut self,
        g: &Generics,
        params: &crate::types::Params,
        lifetimes: &[String],
        on_projections: bool,
    ) -> Result<(Vec<Pred>, Vec<Equality>), Error> {
        let mut out: Vec<Pred> = Vec::new();
        let mut eqs: Vec<(Type, Type)> = Vec::new();
        let push = |out: &mut Vec<Pred>, pred: Pred| {
            if !out.contains(&pred) {
                out.push(pred);
            }
        };
        // What each bound is on, and how it's written, for messages: a type
        // parameter in `<..>`, or a type in a `where` clause, which can be
        // `Self` or a projection too.
        let mut subjects: Vec<(Option<Type>, String, &Vec<BoundItem>)> = g
            .bounds
            .iter()
            .map(|(p, after)| {
                let ty = params
                    .contains(&p.name)
                    .then(|| Type::Param(p.name.clone()));
                (ty, p.name.clone(), after)
            })
            .collect();
        for (subject, after) in &g.wheres {
            let mut lts = Lifetimes::Assoc {
                declared: lifetimes,
            };
            let ty = self.resolve_with(subject, params, &mut lts)?;
            if !bounded(&ty) {
                return Err(Error::ty(
                    subject.span,
                    format!(
                        "a `where` clause can only bound a type parameter, `Self`, or an \
                         associated type of one, not `{ty}`"
                    ),
                ));
            }
            if let Some(l) = after.iter().find(|b| b.name.name.starts_with('\'')) {
                return Err(Error::ty(
                    l.span,
                    "a `where` clause can't say what outlives what (yet); say it in `<..>`",
                ));
            }
            subjects.push((Some(ty.clone()), ty.to_string(), after));
        }
        for (subject_ty, subject_name, after) in subjects {
            for b in after.iter().filter(|b| !b.name.name.starts_with('\'')) {
                let tr = self.trait_named(&b.name)?;
                let Some(ty) = subject_ty.clone() else {
                    continue;
                };
                let subject = Ident {
                    name: subject_name.clone(),
                    span: b.span,
                    id: self.fresh_id(),
                };
                push(
                    &mut out,
                    Pred {
                        ty: ty.clone(),
                        trait_name: tr.clone(),
                    },
                );
                // `F: Fn(i64) -> i64` says what `F`'s calls take and give,
                // as an equality bound on `FnOnce`'s signature.
                let callable = crate::traits::FN_TRAITS.contains(&tr.as_str());
                match (&b.sig, callable) {
                    (Some((ps, ret)), true) => {
                        let unit = TypeExpr {
                            kind: TypeExprKind::Unit,
                            span: b.span,
                            id: self.fresh_id(),
                        };
                        let written = TypeExpr {
                            kind: TypeExprKind::Fn(
                                ps.clone(),
                                Box::new(ret.as_deref().cloned().unwrap_or(unit)),
                                b.effects.clone(),
                            ),
                            span: b.span,
                            id: self.fresh_id(),
                        };
                        let mut lts = Lifetimes::Assoc {
                            declared: lifetimes,
                        };
                        let rhs = self.resolve_with(&written, params, &mut lts)?;
                        let lhs = Type::Proj(
                            Box::new(ty.clone()),
                            FN_ONCE.into(),
                            crate::traits::SIG.into(),
                        );
                        if eqs.iter().any(|(l, _)| *l == lhs) {
                            return Err(Error::ty(
                                b.span,
                                format!(
                                    "`{}` is said to take and give things twice; say it once",
                                    subject.name
                                ),
                            ));
                        }
                        eqs.push((lhs, rhs));
                    }
                    (None, true) => {
                        return Err(Error::ty(
                            b.span,
                            format!(
                                "say what calls of `{0}` take and give, as in `{0}: {tr}(i64) -> \
                                 i64`",
                                subject.name
                            ),
                        ));
                    }
                    (Some(_), false) => {
                        return Err(Error::ty(
                            b.span,
                            format!(
                                "`{tr}` isn't one of `Fn`, `FnMut` and `FnOnce`, so it takes no \
                                 signature"
                            ),
                        ));
                    }
                    (None, false) => {}
                }
                for a in &b.assoc {
                    let owner = match self.traits.assoc_owner(&tr, &a.name.name)[..] {
                        [ref o] => o.clone(),
                        [] => {
                            return Err(Error::ty(
                                a.name.span,
                                format!("trait `{tr}` has no associated type `{}`", a.name.name),
                            ));
                        }
                        ref more => {
                            return Err(Error::ty(
                                a.name.span,
                                format!(
                                    "`{}` is an associated type of several of `{tr}`'s traits: \
                                     {}; bound it on one of those",
                                    a.name.name,
                                    more.join(", ")
                                ),
                            ));
                        }
                    };
                    let lhs = Type::Proj(Box::new(ty.clone()), owner, a.name.name.clone());
                    match &a.kind {
                        AssocBoundKind::Eq(t) => {
                            let mut lts = Lifetimes::Assoc {
                                declared: lifetimes,
                            };
                            let rhs = self.resolve_with(t, params, &mut lts)?;
                            if has_proj(&rhs) {
                                return Err(Error::ty(
                                    t.span,
                                    "an equality bound can only say a projection is a type \
                                     without projections in it (for now)",
                                ));
                            }
                            if eqs.iter().any(|(l, _)| *l == lhs) {
                                return Err(Error::ty(
                                    a.name.span,
                                    format!("`{lhs}` is said to be something twice"),
                                ));
                            }
                            eqs.push((lhs, rhs));
                        }
                        AssocBoundKind::Bounds(traits) => {
                            if !on_projections {
                                return Err(Error::ty(
                                    a.name.span,
                                    format!(
                                        "a bound on an associated type, like `{}: {}`, can only \
                                         be on a function's type parameter: proving an impl's or \
                                         a type's bounds would then be about what `{lhs}` \
                                         stands for, which can be a bigger type",
                                        a.name.name, traits[0].name
                                    ),
                                ));
                            }
                            for t in traits {
                                let t = self.trait_named(t)?;
                                push(
                                    &mut out,
                                    Pred {
                                        ty: lhs.clone(),
                                        trait_name: t,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok((out, eqs))
    }

    /// The type of projection `<s as tr>::name`, written at `span`, or
    /// `s::name` if `tr` isn't written: what it normalises to, or the
    /// projection itself, *rigid*, if it doesn't, as on a type parameter.
    /// `s` must implement the trait, and the shorthand is only for type
    /// parameters (and projections), whose bounds say which trait it means.
    pub(in crate::check) fn projection(
        &self,
        s: Type,
        tr: Option<&Ident>,
        name: &Ident,
        t: &TypeExpr,
        kind: crate::check::Kind,
    ) -> Result<Type, Error> {
        let (n, span) = (&name.name, t.span);
        // What the bounds in scope, and the trait's own for a projection,
        // say `s` implements.
        let traits_of = |s: &Type| -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            let mut bounds: Vec<String> = self
                .env
                .iter()
                .filter(|p| p.ty == *s)
                .map(|p| p.trait_name.clone())
                .collect();
            if let Type::Proj(_, of, a) = s
                && let Some(d) = self.traits.traits[of].assoc.iter().find(|d| d.name == *a)
            {
                bounds.extend(d.bounds.iter().cloned());
            }
            for b in bounds {
                for t in self.traits.implied(&b) {
                    if !out.contains(&t) {
                        out.push(t);
                    }
                }
            }
            out
        };
        let hole = matches!(&s, Type::Param(p) if p.starts_with('?'));
        let tr = match tr {
            Some(tr) => {
                let Some(def) = self.traits.traits.get(&tr.name) else {
                    return Err(Error::ty(tr.span, format!("unknown trait `{}`", tr.name)));
                };
                let owner = self.traits.assoc_owner(&tr.name, n);
                match owner.first() {
                    Some(o) if def.assoc.iter().any(|a| a.name == *n) || owner.len() == 1 => {
                        o.clone()
                    }
                    _ => {
                        return Err(Error::ty(
                            name.span,
                            format!("trait `{}` has no associated type `{n}`", tr.name),
                        ));
                    }
                }
            }
            None if !matches!(s, Type::Param(_) | Type::Proj(..)) || hole => {
                let mut with: Vec<&String> = self
                    .traits
                    .traits
                    .iter()
                    .filter(|(_, d)| d.assoc.iter().any(|a| a.name == *n))
                    .map(|(t, _)| t)
                    .collect();
                with.sort();
                let tr = with.first().map_or("Trait", |t| t.as_str());
                return Err(Error::ty(
                    span,
                    format!(
                        "`{s}::{n}` is short for a projection on a type parameter; on `{s}`, \
                         say which trait's `{n}` it is: write `<{s} as {tr}>::{n}`"
                    ),
                ));
            }
            None => {
                let mut owners: Vec<String> = Vec::new();
                for t in traits_of(&s) {
                    if self.traits.traits[&t].assoc.iter().any(|a| a.name == *n) {
                        owners.push(t);
                    }
                }
                owners.sort();
                match &owners[..] {
                    [o] => o.clone(),
                    [] => {
                        return Err(Error::ty(
                            span,
                            format!("none of `{s}`'s traits has an associated type `{n}`"),
                        ));
                    }
                    more => {
                        let all: Vec<String> = more
                            .iter()
                            .map(|t| format!("`<{s} as {t}>::{n}`"))
                            .collect();
                        return Err(Error::ty(
                            span,
                            format!("`{s}::{n}` could be {}; write which", all.join(" or ")),
                        ));
                    }
                }
            }
        };
        // What it's declared as says what it is, as for a parameter.
        let effect = self.traits.traits[&tr]
            .assoc
            .iter()
            .find(|a| a.name == *n)
            .is_some_and(|a| a.effect);
        match (effect, kind) {
            (true, crate::check::Kind::Type) => {
                return Err(Error::ty(
                    span,
                    format!("`{n}` is an associated effect of `{tr}`, not a type"),
                ));
            }
            (false, crate::check::Kind::Effects) => {
                return Err(Error::ty(
                    span,
                    format!("`{n}` is an associated type of `{tr}`, not an effect"),
                ));
            }
            _ => {}
        }
        if !hole {
            let known = match &s {
                Type::Param(_) | Type::Proj(..) => traits_of(&s).contains(&tr),
                s => {
                    let local = &self.adts.proj.local;
                    self.collecting
                        || crate::check::unify::has_vars(s)
                        || self.traits.implements(s, &tr, &self.env, local).is_ok()
                }
            };
            if !known {
                let fix = match &s {
                    Type::Param(p) => format!("; add a bound: `{p}: {tr}`"),
                    _ => String::new(),
                };
                return Err(Error::ty(
                    span,
                    format!(
                        "`{s}` doesn't implement `{tr}`, so `<{s} as {tr}>::{n}` means nothing{fix}"
                    ),
                ));
            }
        }
        self.proj_traits.borrow_mut().insert(t.id, tr.clone());
        Ok(self.adts.normalise(&Type::Proj(Box::new(s), tr, n.clone())))
    }
}

/// What an equality bound says: projection `.0` is type `.1`.
pub(in crate::check) type Equality = (Type, Type);

/// Can a `where` clause bound `t`: a type parameter, `Self`, or a
/// projection on one? Others' bounds are already true or false.
fn bounded(t: &Type) -> bool {
    match t {
        Type::Param(_) => true,
        Type::Proj(s, ..) => bounded(s),
        _ => false,
    }
}

/// Does `t` have a projection in it?
pub(in crate::check) fn has_proj(t: &Type) -> bool {
    matches!(t, Type::Proj(..)) || t.parts().any(has_proj)
}
