//! Proving obligations, and whether types are well formed, `Copy` or
//! `Discard`.

use super::fields;
use crate::ast::*;
use crate::check::Checker;
use crate::error::{Error, Span};
use crate::traits::{ImplDef, Pred};
use crate::types::{Subst, Type};

impl Checker {
    /// Is `t`, which has no type variables in it, `Copy`, with the bounds
    /// in scope?
    pub(crate) fn is_copy(&self, t: &Type) -> bool {
        self.traits
            .implements(t, "Copy", &self.env, &self.adts.proj.local)
            .is_ok()
    }

    /// May a value of type `t`, which has no type variables in it, be
    /// thrown away, with the bounds in scope? Only asked before
    /// monomorphisation: the borrow checker asks it, and it's done by then.
    pub(crate) fn is_discard(&self, t: &Type) -> bool {
        self.traits
            .implements(t, "Discard", &self.env, &self.adts.proj.local)
            .is_ok()
    }

    /// What `t`, which isn't `Discard`, would need to be, for messages to
    /// say: a type parameter, which a bound can make so, or a struct or
    /// enum that deriving `Discard` would make so, if its fields allow
    /// that. `None` if it's neither, as a fix that wouldn't work is no fix.
    pub(crate) fn missing_discard(&self, t: &Type) -> Option<Type> {
        let bad = self
            .traits
            .implements(t, "Discard", &self.env, &self.adts.proj.local)
            .err()?;
        let Type::Adt(symbol, ..) = &bad.ty else {
            return matches!(bad.ty, Type::Param(_) | Type::Proj(..)).then_some(bad.ty);
        };
        if self.traits.select(&bad.ty, Some("Discard"), &[]).is_some() {
            return None;
        }
        // As if it were `#[derive(Discard)]`, needing it of every type
        // parameter: the derive would need no more.
        let adt = &self.adts[symbol];
        let assumed: Vec<Pred> = adt
            .params
            .iter()
            .map(|p| Pred {
                ty: Type::Param(p.clone()),
                trait_name: "Discard".into(),
            })
            .collect();
        let mut traits = self.traits.clone();
        traits.impls.push(ImplDef {
            span: Span::new(0, 0),
            trait_name: Some("Discard".into()),
            params: adt.params.clone(),
            lifetimes: adt.lifetimes.clone(),
            bounds: adt.bounds.clone(),
            self_ty: adt.generic(symbol),
            preds: assumed.clone(),
            eqs: adt.eqs.clone(),
            assoc: Vec::new(),
            methods: Vec::new(),
            derived: true,
        });
        let fields_may = fields(&adt.kind)
            .iter()
            .all(|(_, f)| traits.implements(f, "Discard", &assumed, &[]).is_ok());
        (fields_may
            && traits
                .implements(t, "Discard", &self.env, &self.adts.proj.local)
                .is_ok())
        .then_some(bad.ty)
    }

    /// Requires `pred`, at `span` because of `why`: now, if its type is
    /// known, or else once the function's types are.
    pub(in crate::check) fn oblige(
        &mut self,
        pred: Pred,
        span: Span,
        why: Why,
    ) -> Result<(), Error> {
        let ty = self.solved(&pred.ty);
        // What a closure implements is known only once its captures are.
        let later = self.has_unsettled_closure(&ty);
        let o = Obligation {
            pred: Pred {
                ty,
                trait_name: pred.trait_name,
            },
            span,
            why,
        };
        match later || crate::check::unify::has_vars(&o.pred.ty) {
            true => {
                self.obligations.push(o);
                Ok(())
            }
            false => self.prove(&o),
        }
    }

    /// Proves obligation `o`, whose type is known, or says why it can't be:
    /// what doesn't implement what, at the bottom, and how to fix that, if
    /// it's simple.
    pub(in crate::check) fn prove(&self, o: &Obligation) -> Result<(), Error> {
        let Pred { ty, trait_name: tr } = &o.pred;
        let bad = match self
            .traits
            .implements(ty, tr, &self.env, &self.adts.proj.local)
        {
            Ok(_) => return Ok(()),
            Err(bad) => bad,
        };
        // How to fix what doesn't hold, if it's simple, or why it can't be.
        let at_bottom = bad == o.pred;
        let fix = |p: &Pred| match (&p.ty, p.trait_name.as_str()) {
            (Type::Param(t), tr) => format!("; add a bound: `{t}: {tr}`"),
            (Type::Fn(_), "Eq") if at_bottom => ": functions can't be compared".to_string(),
            (Type::Fn(_), "Eq") => ", and functions can't be compared".to_string(),
            (Type::Adt(s, ..), _) if crate::types::is_closure(s) => String::new(),
            (t @ Type::Adt(..), tr @ ("Eq" | "Copy" | "Discard")) => {
                format!(
                    "; add `#[derive({tr})]` to `{}`",
                    crate::check::source_name(&t.to_string())
                )
            }
            _ => String::new(),
        };
        // What's at the bottom of it, if it isn't the obligation itself.
        let needs = match at_bottom {
            true => String::new(),
            false => format!("; that needs `{bad}`"),
        };
        let message = match &o.why {
            Why::Compare(op) => match (ty, at_bottom) {
                (Type::Fn(_), _) => format!("cannot compare functions with `{op}`"),
                (Type::Adt(s, ..), _) if crate::types::is_closure(s) => {
                    format!("cannot compare closures with `{op}`")
                }
                (Type::Param(p), true) => format!(
                    "cannot compare values of type `{p}` with `{op}`: `{p}` doesn't implement \
                     `Eq`{}",
                    fix(&bad)
                ),
                (t, true) => format!(
                    "cannot compare `{t}` with `{op}`: it doesn't implement `Eq`{}",
                    match t {
                        Type::Adt(..) => "; add `#[derive(Eq)]` to it",
                        _ => "",
                    }
                ),
                (t, false) => format!(
                    "cannot compare `{t}` with `{op}`: that needs `{bad}`{}",
                    fix(&bad)
                ),
            },
            Why::Bound(what, p) => format!(
                "`{what}` needs its `{p}` to implement `{tr}`, and `{ty}` doesn't{needs}{}",
                fix(&bad)
            ),
            Why::Method(m) => format!(
                "`{ty}` doesn't implement `{tr}`, which method `{m}` is of{needs}{}",
                fix(&bad)
            ),
            Why::Call => format!("`{ty}` can't be called{needs}"),
            Why::Integer => format!(
                "`{ty}` doesn't implement `{tr}`, so it isn't an integer type, which this needs{}",
                fix(&bad)
            ),
            Why::TryBranch => format!(
                "`?` needs a `Result`, an `Option` or another type that implements `{tr}`, and \
                 `{ty}` doesn't{needs}{}",
                fix(&bad)
            ),
            Why::TryReturn(who) => format!(
                "`?` returns early from {who}, but its result, `{ty}`, doesn't implement \
                 `{tr}`{needs}{}",
                fix(&bad)
            ),
            Why::Returns(f) if *ty == Type::Never => format!(
                "can't tell what type `{f}` returns: it never returns, so nothing says which \
                 type this is"
            ),
            Why::Returns(f) => format!(
                "`{ty}` doesn't implement `{tr}`, which `{f}`'s return type says it does{needs}{}",
                fix(&bad)
            ),
        };
        // A closure that implements less than it's needed to: why.
        let message = match (&o.why, self.closure_why(&bad.ty)) {
            (Why::Bound(what, _), Some((kind, why)))
                if crate::traits::FN_TRAITS.contains(&bad.trait_name.as_str()) =>
            {
                format!(
                    "this closure is only `{}`, as {why}, but `{what}` needs it to implement \
                     `{}`",
                    kind.name(),
                    bad.trait_name
                )
            }
            (Why::Returns(f), Some((kind, why)))
                if crate::traits::FN_TRAITS.contains(&bad.trait_name.as_str()) =>
            {
                format!(
                    "this closure is only `{}`, as {why}, but `{f}`'s return type says it \
                     implements `{}`",
                    kind.name(),
                    bad.trait_name
                )
            }
            _ => message,
        };
        Err(Error::ty(o.span, message))
    }

    /// At the end of a function, the obligations that waited for its types.
    pub(in crate::check) fn prove_all(&mut self) -> Result<(), Error> {
        for mut o in std::mem::take(&mut self.obligations) {
            o.pred.ty = self.decided_regions(&self.solved(&o.pred.ty));
            // If it's still not fully known, that's reported later.
            if !crate::check::unify::has_vars(&o.pred.ty) {
                self.prove(&o)?;
            }
        }
        Ok(())
    }

    /// Requires what struct or enum types in `t` need of their type
    /// arguments, by their bounds: `S<i64>` for `struct S<T: Show>` needs
    /// `i64: Show`.
    pub(in crate::check) fn well_formed(&mut self, t: &Type, span: Span) -> Result<(), Error> {
        let mut preds = Vec::new();
        let mut eqs = Vec::new();
        crate::check::each_adt(t, &mut |name, args| {
            if let Some(def) = self.adts.get(name) {
                let s = Subst::new(&def.params, args);
                for p in &def.preds {
                    let why = Why::Bound(crate::check::source_name(name), p.ty.to_string());
                    let pred = Pred {
                        ty: s.apply(&p.ty),
                        trait_name: p.trait_name.clone(),
                    };
                    preds.push((pred, why));
                }
                for (lhs, rhs) in &def.eqs {
                    eqs.push((s.apply(lhs), s.apply(rhs), crate::check::source_name(name)));
                }
            }
        });
        for (pred, why) in preds {
            self.oblige(pred, span, why)?;
        }
        for (lhs, rhs, who) in eqs {
            self.require_eq(lhs, rhs, span, &who)?;
        }
        Ok(())
    }

    /// `well_formed`, for a type with no variables in it, outside a body.
    pub(in crate::check) fn well_formed_now(&mut self, t: &Type, span: Span) -> Result<(), Error> {
        self.well_formed(t, span)
    }
}

/// Why the checker has to prove an obligation, for saying what's wrong if
/// it can't.
#[derive(Debug, Clone)]
pub(in crate::check) enum Why {
    /// `==` or `!=` (`.0`) on a value of the type.
    Compare(BinOp),
    /// A bound of what's named `.0`, on its type parameter `.1`.
    Bound(String, String),
    /// A call of method `.0` of the trait.
    Method(String),
    /// A call of a value of the type.
    Call,
    /// The return type of function `.0`, an opaque type, says it does.
    Returns(String),
    /// It's an integer literal's type, or an operator's operands'.
    Integer,
    /// What a `?` is of.
    TryBranch,
    /// What `.0`, a function or closure, returns, which a `?` in it
    /// returns early as.
    TryReturn(String),
}

/// An obligation the checker has to prove once the types in it are known.
#[derive(Debug, Clone)]
pub(in crate::check) struct Obligation {
    pub(in crate::check) pred: Pred,
    pub(in crate::check) span: Span,
    pub(in crate::check) why: Why,
}
