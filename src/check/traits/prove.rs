//! Proving obligations, and whether types are well formed, `Copy` or
//! `Discard`.

use super::fields;
use crate::ast::*;
use crate::check::Checker;
use crate::error::{Error, Span};
use crate::traits::{COPY, DEBUG, DISCARD, EQ};
use crate::traits::{ImplDef, Pred};
use crate::types::{Subst, Type};

impl Checker {
    /// Is `t`, which has no type variables in it, `Copy`, with the bounds
    /// in scope?
    pub(crate) fn is_copy(&self, t: &Type) -> bool {
        self.traits
            .implements(t, COPY, &self.adts.proj.env, &self.adts.proj.local)
            .is_ok()
    }

    /// Is `call` the call of `IntoIterator::into_iter` a `for` makes of its
    /// head? Of an iterator, it's the head itself.
    pub(crate) fn is_for_head(&self, call: NodeId) -> bool {
        self.modules.for_heads.contains(&call)
    }

    /// Is `t`, which has no type variables in it, an iterator, with the
    /// bounds in scope?
    pub(crate) fn implements_iterator(&self, t: &Type) -> bool {
        (self.traits)
            .implements(
                t,
                crate::check::ITERATOR,
                &self.adts.proj.env,
                &self.adts.proj.local,
            )
            .is_ok()
    }

    /// May a value of type `t`, which has no type variables in it, be
    /// thrown away, with the bounds in scope? Only asked before
    /// monomorphisation: the borrow checker asks it, and it's done by then.
    pub(crate) fn is_discard(&self, t: &Type) -> bool {
        self.traits
            .implements(t, DISCARD, &self.adts.proj.env, &self.adts.proj.local)
            .is_ok()
    }

    /// What `t`, which isn't `Discard`, would need to be, for messages to
    /// say: a type parameter, which a bound can make so, or a struct or
    /// enum that deriving `Discard` would make so, if its fields allow
    /// that. `None` if it's neither, as a fix that wouldn't work is no fix.
    pub(crate) fn missing_discard(&self, t: &Type) -> Option<Type> {
        let bad = self
            .traits
            .implements(t, DISCARD, &self.adts.proj.env, &self.adts.proj.local)
            .err()?;
        let Type::Adt(symbol, ..) = &bad.ty else {
            return matches!(bad.ty, Type::Param(_, _) | Type::Proj(..)).then_some(bad.ty);
        };
        if self.traits.select(&bad.ty, Some(DISCARD), &[]).is_some() {
            return None;
        }
        // As if it were `#[derive(Discard)]`, needing it of every type
        // parameter: the derive would need no more.
        let adt = &self.adts[symbol];
        let assumed: Vec<Pred> = adt
            .params
            .iter()
            .map(|p| Pred {
                ty: adt.params.ty(p),
                trait_name: DISCARD.into(),
            })
            .collect();
        let mut traits = self.traits.clone();
        traits.impls.push(ImplDef {
            span: Span::new(0, 0),
            trait_name: Some(DISCARD.into()),
            params: adt.params.clone(),
            lifetimes: adt.lifetimes.clone(),
            bounds: adt.bounds.clone(),
            self_ty: adt.generic(symbol),
            preds: assumed.clone(),
            eqs: adt.eqs.clone(),
            effect_bounds: Vec::new(),
            assoc: Vec::new(),
            methods: Vec::new(),
            derived: true,
        });
        let fields_may = fields(&adt.kind)
            .iter()
            .all(|(_, f)| traits.implements(f, DISCARD, &assumed, &[]).is_ok());
        (fields_may
            && traits
                .implements(t, DISCARD, &self.adts.proj.env, &self.adts.proj.local)
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
            .implements(ty, tr, &self.adts.proj.env, &self.adts.proj.local)
        {
            Ok(_) => return Ok(()),
            Err(bad) => bad,
        };
        // How to fix what doesn't hold, if it's simple, or why it can't be.
        let at_bottom = bad == o.pred;
        let fix = |p: &Pred| match (&p.ty, p.trait_name.as_str()) {
            (Type::Param(t, _), tr) => format!("; add a bound: `{t}: {}`", tr),
            (Type::Fn(_), EQ) if at_bottom => ": functions can't be compared".to_string(),
            (Type::Fn(_), EQ) => ", and functions can't be compared".to_string(),
            (Type::Adt(s, ..), _) if crate::types::is_closure(s) => String::new(),
            (t @ Type::Adt(..), tr @ (EQ | COPY | DISCARD | DEBUG)) => {
                format!(
                    "; add `#[derive({})]` to `{}`",
                    tr.rsplit("::").next().unwrap_or(tr),
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
            // A hole of a format string, which is a call of `display` or
            // `debug` (`modules::Rewrite::format`).
            Why::Bound(f, _)
                if (f == "core::fmt::hex" || f == "core::fmt::dec")
                    && self.modules.formats.contains(&o.span) =>
            {
                let hole = if f == "core::fmt::hex" {
                    "{x:x}"
                } else {
                    "{x:05}"
                };
                format!(
                    "`{ty}` isn't an integer type, which a hole `{hole}` of a format string \
                     needs{needs}"
                )
            }
            Why::Bound(f, _) if self.modules.formats.contains(&o.span) && f != "core::fmt::cat" => {
                let (name, hole) = match tr.as_str() {
                    DEBUG => ("Debug", "`{x:?}`"),
                    _ => ("Display", "`{x}`"),
                };
                format!(
                    "`{ty}` doesn't implement `{name}`, which a hole {hole} of a format string \
                     needs{needs}{}",
                    fix(&bad)
                )
            }
            Why::Compare(op) => match (ty, at_bottom) {
                (Type::Fn(_), _) => format!("cannot compare functions with `{op}`"),
                (Type::Adt(s, ..), _) if crate::types::is_closure(s) => {
                    format!("cannot compare closures with `{op}`")
                }
                (Type::Param(p, _), true) => format!(
                    "cannot compare values of type `{p}` with `{op}`: `{p}` doesn't implement \
                     `{tr}`{}",
                    fix(&bad)
                ),
                (t, true) => format!(
                    "cannot compare `{t}` with `{op}`: it doesn't implement `{tr}`{}",
                    match (t, o.pred.trait_name.as_str()) {
                        (Type::Adt(..), EQ) => format!("; add `#[derive({tr})]` to it"),
                        _ => String::new(),
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
            // At the bottom, it's the type itself that's neither, by the
            // blanket impl's bound or not; else it would be, but for what's
            // at the bottom.
            Why::ForHead if bad.ty == *ty => format!(
                "`{ty}` can't be iterated: it isn't an iterator, nor `IntoIterator`{}",
                fix(&bad)
            ),
            Why::ForHead => format!("`{ty}` can't be iterated: that needs `{bad}`{}", fix(&bad)),
            Why::Repeat => format!(
                "`[v; n]` copies `v`, but `{ty}` isn't `{tr}`{needs}{}",
                fix(&bad)
            ),
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
    /// Of those that don't hold, the one reported is the first nearest the
    /// cause: one that fails itself, rather than by what it needs; else one
    /// of a hole of an `f".."`, where the value was written, rather than of
    /// the call the whole string is passed to; else the first.
    ///
    /// It's done twice: once its types are known (`early`), before the
    /// borrow checker, which can take for granted what they say, for all
    /// but those that wait for its effects; and once those are settled,
    /// for the rest (see `waits_for_effects`).
    pub(in crate::check) fn prove_all(&mut self, early: bool) -> Result<(), Error> {
        let mut found: Option<(u8, Error)> = None;
        for mut o in std::mem::take(&mut self.obligations) {
            o.pred.ty = self.solved(&o.pred.ty);
            if early && self.waits_for_effects(&o.pred.ty) {
                self.obligations.push(o);
                continue;
            }
            // If it's still not fully known, that's reported later.
            if crate::check::unify::has_vars(&o.pred.ty) {
                continue;
            }
            if let Err(e) = self.prove(&o) {
                let (ty, tr) = (&o.pred.ty, &o.pred.trait_name);
                let local = &self.adts.proj.local;
                let rank = match self.traits.implements(ty, tr, &self.adts.proj.env, local) {
                    Err(bad) if bad == o.pred => return Err(e),
                    _ if self.modules.formats.contains(&o.span)
                        && !matches!(&o.why, Why::Bound(f, _) if f == "core::fmt::cat") =>
                    {
                        1
                    }
                    _ => 2,
                };
                if found.as_ref().is_none_or(|(r, _)| rank < *r) {
                    found = Some((rank, e));
                }
            }
        }
        found.map_or(Ok(()), |(_, e)| Err(e))
    }

    /// Does whether `t` implements a trait wait for the function's effects
    /// to be settled: has it an effect variable in it, or a variable at
    /// all, which may turn out to be or have one, or a closure whose
    /// signature or fields' types have? An impl may be of a type with some
    /// effects and not others, `fn() -> i64`, which can't panic, or need
    /// of one that its calls are, `F: Fn() -> U`.
    fn waits_for_effects(&self, t: &Type) -> bool {
        use crate::check::unify::has_vars;
        let mut waits = has_vars(t);
        crate::check::each_adt(t, &mut |name, _| {
            waits |= (self.traits.closures.get(name)).is_some_and(|c| {
                has_vars(&self.solved(&Type::Fn(c.sig.clone())))
                    || (c.fields.iter()).any(|(_, f)| has_vars(&self.solved(f)))
            });
        });
        waits
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
    /// What's copied by `[v; n]`.
    Repeat,
    /// What a `for` goes through, which it calls `into_iter` of
    /// (`modules::Rewrite::for_loop`).
    ForHead,
}

/// An obligation the checker has to prove once the types in it are known.
#[derive(Debug, Clone)]
pub(in crate::check) struct Obligation {
    pub(in crate::check) pred: Pred,
    pub(in crate::check) span: Span,
    pub(in crate::check) why: Why,
}
