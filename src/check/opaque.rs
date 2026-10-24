//! Opaque types, `impl Trait` in a return type (see
//! docs/06c.1-opaque-types.md): what each stands for, and that none leads
//! back to itself.

use super::{Checker, each_adt, traits};
use crate::ast::*;
use crate::error::Error;
use crate::traits::Pred;
use crate::types::{Subst, Type};
use std::collections::HashSet;

impl Checker {
    /// Defines the opaque type `t`, an `impl Tr` written in the return type
    /// of the function whose signature is being worked out, which has type
    /// parameters `params` and lifetimes `lifetimes`. Its bounds are those
    /// of a type parameter, with it for the parameter.
    pub(super) fn opaque(
        &mut self,
        t: &TypeExpr,
        params: &crate::types::Params,
        lifetimes: &[String],
    ) -> Result<(), Error> {
        let TypeExprKind::Impl(bounds) = &t.kind else {
            unreachable!("an opaque type is written `impl`")
        };
        let symbol = crate::types::opaque_symbol(&t.to_string(), t.span.start);
        let subject = Ident {
            name: symbol.clone(),
            span: t.span,
            id: t.id,
        };
        let g = Generics {
            bounds: vec![(subject, bounds.clone())],
            ..Generics::default()
        };
        let mut with_it = params.clone();
        with_it.push_type(symbol.clone());
        let (preds, eqs) = self.preds(&g, &with_it, lifetimes, true)?;
        let (owner, owner_name) = self.sig_owner.clone();
        let mut def = crate::traits::OpaqueDef {
            owner,
            owner_name,
            params: params.clone(),
            lifetimes: lifetimes.to_vec(),
            preds: Vec::new(),
            eqs: Vec::new(),
            hidden: None,
            span: t.span,
        };
        let s = Subst::new(std::slice::from_ref(&symbol), &[def.generic(&symbol)]);
        def.preds = preds
            .into_iter()
            .map(|p| Pred {
                ty: s.apply(&p.ty),
                trait_name: p.trait_name,
            })
            .collect();
        def.eqs = eqs
            .into_iter()
            .map(|(l, r)| (s.apply(&l), s.apply(&r)))
            .collect();
        self.opaque_at.insert(t.id, symbol.clone());
        self.adts.proj.opaques.insert(symbol.clone(), def.clone());
        self.traits.opaques.insert(symbol, def);
        Ok(())
    }

    /// The symbols of the opaque types a value of type `t` may hold: in
    /// it, and in the types of what the closures in it capture.
    fn opaques_held(&self, t: &Type) -> Vec<String> {
        let mut out = opaques_of(t);
        let mut seen = HashSet::new();
        let mut todo = vec![t.clone()];
        while let Some(t) = todo.pop() {
            each_adt(&t, &mut |name, _| {
                if let Some(c) = self.traits.closures.get(name)
                    && seen.insert(name.to_string())
                {
                    for (_, f) in &c.fields {
                        out.extend(opaques_of(f));
                        todo.push(f.clone());
                    }
                }
            });
        }
        out
    }

    /// Rejects opaque types that stand for types that hold them, through
    /// others or not: `fn f() -> impl Fn() -> i64` whose closure holds
    /// what `f()` returns would be a type that holds itself, which no
    /// finite type is. Then replacing each with what it stands for, as
    /// monomorphisation does (`Projections::reveal`), ends.
    ///
    /// Only the package being checked's need looking at: an earlier
    /// package's can't hold its, which they can't name.
    pub(super) fn opaque_cycles(&self) -> Result<(), Error> {
        let opaques = &self.traits.opaques;
        let mut symbols: Vec<&String> = (opaques.keys())
            .filter(|s| self.current(opaques[*s].span))
            .collect();
        symbols.sort_by_key(|s| self.traits.opaques[*s].span.start);
        for start in symbols {
            let mut todo = vec![start.clone()];
            let mut seen = HashSet::new();
            while let Some(s) = todo.pop() {
                let Some(hidden) = &self.traits.opaques[&s].hidden else {
                    continue;
                };
                for o in self.opaques_held(hidden) {
                    if o == *start {
                        let d = &self.traits.opaques[start];
                        return Err(Error::ty(
                            d.span,
                            format!(
                                "the type `{}` returns would contain itself: what it returns \
                                 holds what {} returns",
                                d.owner_name,
                                match s == *start {
                                    true => "it".to_string(),
                                    false => format!("`{}`", self.traits.opaques[&s].owner_name),
                                }
                            ),
                        ));
                    }
                    if seen.insert(o.clone()) {
                        todo.push(o);
                    }
                }
            }
        }
        Ok(())
    }

    /// `ret`, the return type of function `symbol`, called `name`, with
    /// each of its opaque types replaced by a new variable, for the type it
    /// stands for, which `hidden` gets with the opaque type's symbol. Each
    /// must turn out to have what the bounds say.
    pub(super) fn hide(
        &mut self,
        ret: &Type,
        symbol: &str,
        name: &str,
        hidden: &mut Vec<(String, Type)>,
    ) -> Result<Type, Error> {
        let mut parts = Vec::new();
        for p in ret.parts() {
            parts.push(self.hide(p, symbol, name, hidden)?);
        }
        let mut parts = parts.into_iter();
        let t = ret.map(|_| parts.next().expect("as many"));
        let Type::Opaque(s, ..) = &t else {
            return Ok(t);
        };
        let d = self.traits.opaques[s].clone();
        if d.owner != symbol {
            return Ok(t);
        }
        if let Some((_, var)) = hidden.iter().find(|(h, _)| h == s) {
            return Ok(var.clone());
        }
        let var = self.fresh(
            d.span,
            format!("the type `{name}` returns"),
            "nothing in its body says".to_string(),
        );
        let swap = |u: &Type| replace_type(u, &t, &var);
        for p in &d.preds {
            let pred = Pred {
                ty: swap(&p.ty),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, d.span, traits::Why::Returns(name.to_string()))?;
        }
        for (lhs, rhs) in &d.eqs {
            self.require_eq(
                swap(lhs),
                rhs.clone(),
                d.span,
                &format!("{name}'s return type"),
            )?;
        }
        hidden.push((s.clone(), var.clone()));
        Ok(var)
    }
}

/// Every `impl Tr` written in return type `t`, where they can be: not in a
/// function type, which would be a different type for each function, nor
/// in another's bounds, nor a projection.
pub(super) fn opaques_in<'t>(t: &'t TypeExpr, out: &mut Vec<&'t TypeExpr>) {
    match &t.kind {
        TypeExprKind::Impl(_) => out.push(t),
        TypeExprKind::Named(_, args) => args.iter().for_each(|a| opaques_in(a, out)),
        TypeExprKind::Ref(_, _, inner)
        | TypeExprKind::Slice(inner)
        | TypeExprKind::Refined(_, inner, _) => opaques_in(inner, out),
        TypeExprKind::Fn(..)
        | TypeExprKind::Proj(..)
        | TypeExprKind::Unit
        | TypeExprKind::Never
        | TypeExprKind::Lifetime(_) => {}
    }
}

/// The symbols of the opaque types in `t`.
pub(super) fn opaques_of(t: &Type) -> Vec<String> {
    let mut out = Vec::new();
    if let Type::Opaque(s, ..) = t {
        out.push(s.clone());
    }
    for p in t.parts() {
        out.extend(opaques_of(p));
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
