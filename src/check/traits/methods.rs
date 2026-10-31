//! Finding the method a call `x.m()` or a path `T::m` names, and which
//! methods a use of a trait method may run (`dispatch`).

use super::Why;
use crate::ast::*;
use crate::check::Checker;
use crate::error::{Edit, Error, Fix, Span};
use crate::traits::Pred;
use crate::traits::{DISCARD, EQ};
use crate::types::{Subst, Type};
use std::collections::HashMap;

impl Checker {
    /// The method `name` of values of type `ty`, which must be known, for
    /// `x.name(..)`. See `lookup_method`.
    pub(in crate::check) fn method_of(
        &mut self,
        ty: &Type,
        name: &Ident,
        targs: &[TypeExpr],
    ) -> Result<Found, Error> {
        self.lookup_method(ty, None, name, targs, "value")
    }

    /// The symbol of the method `name` of the one inherent impl that could
    /// be for `ty`, if there's one: for a refinement's `x.len()`, which can
    /// only call a measure, which is one (see `refine::find_measure`).
    pub(in crate::check) fn inherent_named(&self, ty: &Type, name: &str) -> Option<String> {
        let ty = self.solved(ty);
        let mut found = self
            .traits
            .impls
            .iter()
            .filter(|d| d.trait_name.is_none() && could_match(&d.self_ty, &ty, &d.params))
            .filter_map(|d| d.methods.iter().find(|(n, _)| n == name));
        match (found.next(), found.next()) {
            (Some((_, symbol)), None) => Some(symbol.clone()),
            _ => None,
        }
    }

    /// The method `name` of type `ty`, or of trait `tr` at `Self` = `ty`.
    ///
    /// An inherent method comes first: one in an `impl` of the type itself.
    /// Then a trait's: for a type parameter, of a trait its bounds say it
    /// implements; for any other type, of a trait some impl could be for
    /// it. Two traits with a method of that name are ambiguous, and the
    /// path `Trait::name(x)` says which. Which impl of the trait it is
    /// isn't decided here: the obligation that `ty` implements the trait
    /// is, and monomorphisation finds the impl.
    pub(in crate::check) fn lookup_method(
        &mut self,
        ty: &Type,
        tr: Option<&str>,
        name: &Ident,
        targs: &[TypeExpr],
        what: &str,
    ) -> Result<Found, Error> {
        let ty = self.solved(ty);
        let m = &name.name;
        // The traits it implements that aren't in scope, if it's a type
        // whose impls are looked at.
        let mut hidden: Vec<String> = Vec::new();
        if tr.is_none() {
            let inherent: Vec<usize> = self
                .traits
                .impls
                .iter()
                .enumerate()
                .filter(|(_, d)| d.trait_name.is_none() && d.methods.iter().any(|(n, _)| n == m))
                .filter(|(_, d)| could_match(&d.self_ty, &ty, &d.params))
                .map(|(i, _)| i)
                .collect();
            match inherent[..] {
                [i] => return self.inherent_method(i, &ty, name, targs),
                [] => {}
                _ => {
                    return Err(Error::ty(
                        name.span,
                        format!(
                            "several impls of `{ty}` have a method `{m}`, so which one this is \
                             depends on what `{ty}` is; annotate it"
                        ),
                    ));
                }
            }
        }
        let traits: Vec<String> = match (tr, &ty) {
            (Some(tr), _) => vec![tr.to_string()],
            (None, Type::Param(_) | Type::Proj(..)) => {
                let mut bounds: Vec<String> = self
                    .env
                    .iter()
                    .filter(|p| p.ty == ty)
                    .map(|p| p.trait_name.clone())
                    .collect();
                // What the trait says of its associated type, for a
                // projection.
                if let Type::Proj(_, of, a) = &ty
                    && let Some(d) = self.traits.traits[of].assoc.iter().find(|d| d.name == *a)
                {
                    bounds.extend(d.bounds.iter().cloned());
                }
                let mut out = Vec::new();
                for b in bounds {
                    for t in self.traits.implied(&b) {
                        if !out.contains(&t) {
                            out.push(t);
                        }
                    }
                }
                out
            }
            (None, _) => {
                let mut out: Vec<String> = self
                    .traits
                    .impls
                    .iter()
                    .filter_map(|d| {
                        d.trait_name
                            .clone()
                            .filter(|_| could_match(&d.self_ty, &ty, &d.params))
                    })
                    .collect();
                if builtin_eq(&ty) {
                    out.push(EQ.into());
                }
                if builtin_discard(&ty) {
                    out.push(DISCARD.into());
                }
                // What an opaque type's bounds say it implements.
                if let Type::Opaque(symbol, ..) = &ty {
                    for p in &self.traits.opaques[symbol].preds {
                        if let Type::Opaque(..) = p.ty {
                            out.extend(self.traits.implied(&p.trait_name));
                        }
                    }
                }
                out.sort();
                out.dedup();
                // Only a trait in scope here: what one in another module
                // says can't change what a call here means.
                let (visible, out): (Vec<String>, Vec<String>) = out
                    .into_iter()
                    .partition(|t| self.modules.trait_in_scope(self.module, t));
                hidden = out;
                visible
            }
        };
        let mut having: Vec<&str> = traits
            .iter()
            .filter(|t| self.traits.method(t, m).is_some())
            .map(String::as_str)
            .collect();
        having.sort();
        let tr = match having[..] {
            [tr] => tr.to_string(),
            [] if let Some(h) = hidden.iter().find(|h| self.traits.method(h, m).is_some()) => {
                let path = self.modules.path_to_trait(self.module, h);
                let fix = Fix::new(
                    format!("import `{h}`"),
                    vec![Edit::insert(
                        self.modules.ends[self.module],
                        format!("\nuse {path};\n"),
                    )],
                );
                return Err(Error::ty(
                    name.span,
                    format!(
                        "`{ty}` has no method `{m}` in scope; trait `{h}` has one: import it \
                         with `use {path};`"
                    ),
                )
                .with_fix(fix));
            }
            [] => {
                let message = match (tr, &ty) {
                    (Some(tr), _) => format!("trait `{tr}` has no method `{m}`"),
                    (None, Type::Param(p)) => {
                        let mut with: Vec<&String> = self
                            .traits
                            .traits
                            .iter()
                            .filter(|(_, d)| d.methods.iter().any(|x| x.name == *m))
                            .map(|(t, _)| t)
                            .collect();
                        with.sort();
                        match with.first() {
                            Some(t) => format!(
                                "`{p}` has no method `{m}`: its bounds don't give it one; \
                                 `{t}` has one, so a bound `{p}: {t}` would"
                            ),
                            None => format!("`{p}` has no method `{m}`"),
                        }
                    }
                    (None, t) => {
                        let mut with: Vec<&String> = self
                            .traits
                            .traits
                            .iter()
                            .filter(|(_, d)| d.methods.iter().any(|x| x.name == *m))
                            .map(|(t, _)| t)
                            .collect();
                        with.sort();
                        match with.first() {
                            Some(tr) => format!(
                                "`{t}` has no method `{m}`; trait `{tr}` has one, but `{t}` \
                                 doesn't implement it"
                            ),
                            None => format!("`{t}` has no method `{m}`"),
                        }
                    }
                };
                return Err(Error::ty(name.span, message));
            }
            _ => {
                let all: Vec<String> = having.iter().map(|t| format!("`{t}`")).collect();
                return Err(Error::ty(
                    name.span,
                    format!(
                        "method `{m}` of this {what} is ambiguous: traits {} both have one; say \
                         which, as in `{}::{m}(..)`",
                        all.join(" and "),
                        having[0]
                    ),
                ));
            }
        };
        self.modules.use_trait(self.module, &tr);
        let def = self.traits.method(&tr, m).expect("it has it").clone();
        let own = def.scheme.params.skip(1);
        let mut args = vec![ty.clone()];
        args.extend(self.type_args(m, &own, targs, name.span, "(..)")?);
        let fn_ty = Subst::new(&def.scheme.params, &args).apply(&def.scheme.ty);
        // Its own bounds, besides `Self: Trait`.
        let s = Subst::new(&def.scheme.params, &args);
        for p in &def.scheme.preds[1..] {
            let why = Why::Bound(m.clone(), p.ty.to_string());
            let pred = Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, name.span, why)?;
        }
        for (lhs, rhs) in &def.scheme.eqs {
            self.require_eq(s.apply(lhs), s.apply(rhs), name.span, m)?;
        }
        Ok(Found {
            symbol: def.symbol.clone(),
            ty: fn_ty,
            args,
            of_trait: Some(Pred { ty, trait_name: tr }),
            def: def.span,
            receiver: def.receiver,
        })
    }

    /// Method `name` of inherent impl `i`, for type `ty`.
    fn inherent_method(
        &mut self,
        i: usize,
        ty: &Type,
        name: &Ident,
        targs: &[TypeExpr],
    ) -> Result<Found, Error> {
        let d = self.traits.impls[i].clone();
        let symbol = d
            .methods
            .iter()
            .find(|(n, _)| *n == name.name)
            .expect("it has it")
            .1
            .clone();
        let id = self
            .fns
            .iter()
            .position(|f| f.symbol == symbol)
            .expect("a method");
        self.uses().fns.insert(crate::check::scope::FnId(id));
        let f = &self.fns[id];
        if !self.modules.visible(f.module, f.level, self.module) {
            return Err(Error::ty(
                name.span,
                format!(
                    "method `{}` of `{}` {}",
                    name.name,
                    d.self_ty,
                    self.modules.hidden_by(f.module, f.level)
                ),
            ));
        }
        let (scheme, def) = (f.scheme.clone(), f.name.span);
        let receiver = self.receivers[id];
        // The impl's parameters are worked out from the type; the method's
        // own are written or inferred.
        let mut args: Vec<Type> = d
            .params
            .iter()
            .map(|p| {
                self.fresh_param(
                    &d.params,
                    p,
                    name.span,
                    format!("`{p}` of the impl of `{}`", d.self_ty),
                    "annotate the type".into(),
                )
            })
            .collect();
        let own = scheme.params.skip(d.params.len());
        args.extend(self.type_args(&name.name, &own, targs, name.span, "(..)")?);
        let s = Subst::new(&scheme.params, &args);
        let self_ty = s.apply(&d.self_ty);
        self.unify(&self_ty, ty)
            .unwrap_or_else(|_| unreachable!("it could match"));
        let who = format!("{}::{}", d.self_ty, name.name);
        for p in &scheme.preds {
            let why = Why::Bound(who.clone(), p.ty.to_string());
            let pred = Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, name.span, why)?;
        }
        for (lhs, rhs) in &scheme.eqs {
            self.require_eq(s.apply(lhs), s.apply(rhs), name.span, &who)?;
        }
        Ok(Found {
            symbol,
            ty: s.apply(&scheme.ty),
            args,
            of_trait: None,
            def: Some(def),
            receiver,
        })
    }
}

/// The method a call or path names.
pub(in crate::check) struct Found {
    /// What the call names: the method's symbol.
    pub(in crate::check) symbol: String,
    /// Its type, with its type parameters, all of them, replaced by `args`.
    pub(in crate::check) ty: Type,
    pub(in crate::check) args: Vec<Type>,
    /// The trait it's a method of, if it is, and the type that implements
    /// it, which must.
    pub(in crate::check) of_trait: Option<Pred>,
    /// Where it's declared, if anywhere.
    pub(in crate::check) def: Option<Span>,
    pub(in crate::check) receiver: bool,
}

/// The symbol of method `m` of trait `tr`: `<Show>::show`.
pub(in crate::check) fn trait_method_symbol(tr: &str, m: &str) -> String {
    format!("<{tr}>::{m}")
}

/// Could `pat`, which mentions type parameters `params`, be `t`, which may
/// have type variables in it, for some choice of the parameters and the
/// variables?
fn could_match(pat: &Type, t: &Type, params: &[String]) -> bool {
    let mut found = HashMap::new();
    could_match_in(pat, t, params, &mut found)
}

fn could_match_in(
    pat: &Type,
    t: &Type,
    params: &[String],
    found: &mut HashMap<String, Type>,
) -> bool {
    match (pat, t) {
        (_, Type::Var(_)) => true,
        (Type::Param(p), _) if params.contains(p) => match found.get(p) {
            Some(prev) => {
                let prev = prev.clone();
                could_match_in(&prev, t, &[], &mut HashMap::new())
            }
            None => {
                found.insert(p.clone(), t.clone());
                true
            }
        },
        _ if pat.same_shape(t) => pat
            .parts()
            .zip(t.parts())
            .all(|(p, q)| could_match_in(p, q, params, found)),
        _ => false,
    }
}

/// Does `t` implement `Eq` by a built-in impl, whatever it holds?
/// Could `t` be `Discard` by a built-in rule: a type of the language's, or
/// a closure? Then `x.discard()` is `Discard`'s, which a bound proves.
fn builtin_discard(t: &Type) -> bool {
    match t {
        Type::Adt(symbol, ..) => crate::types::is_closure(symbol),
        Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never => true,
        Type::Fn(_) | Type::Ref(..) | Type::Box(_) => true,
        _ => false,
    }
}

fn builtin_eq(t: &Type) -> bool {
    matches!(
        t,
        Type::Int(_) | Type::Bool | Type::Unit | Type::Never | Type::Ref(..) | Type::Box(_)
    )
}
