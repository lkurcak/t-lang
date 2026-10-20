//! Collecting traits and impls, and checking the rules that keep trait
//! resolution simple (see `traits.rs`): every impl makes sense, no two
//! overlap (*coherence*), and proving that a type implements a trait always
//! ends (*termination*). Also proving obligations, and finding the method a
//! call `x.m()` or a path `T::m` names.

use super::generics::{self, Lifetimes};
use super::{Checker, plural};
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::parser::is_effect_name;
use crate::traits::{AssocDecl, ImplDef, MethodDef, Pred, TraitDef, eq_subject, overlap};
use crate::types::{AdtKind, Bound, Region, Scheme, Subst, Type};
use std::collections::{HashMap, HashSet};

/// What a method of an impl is inside of: the impl's parameters, bounds
/// and equality bounds, which it's generic over too. Nothing, in a trait.
#[derive(Debug, Clone, Default)]
pub(super) struct Outer {
    pub(super) params: Vec<String>,
    pub(super) lifetimes: Vec<String>,
    pub(super) bounds: Vec<Bound>,
    pub(super) preds: Vec<Pred>,
    pub(super) eqs: Vec<(Type, Type)>,
}

/// What an equality bound says: projection `.0` is type `.1`.
pub(super) type Equality = (Type, Type);

/// A stand-in for a projection, for `dispatch`: its name, the projection,
/// and which parameters of the function it's in the projection mentions.
type Hidden = (String, Type, Vec<usize>);

/// A use in `assoc_regular`: from what, to what, with what arguments, what
/// of `to`'s parameters is inside `from`'s, and where.
type Edge<'a> = (String, &'a str, Vec<Type>, Vec<(usize, usize, usize)>, Span);

/// Does `t` have a projection in it?
/// Can a `where` clause bound `t`: a type parameter, `Self`, or a
/// projection on one? Others' bounds are already true or false.
fn bounded(t: &Type) -> bool {
    match t {
        Type::Param(_) => true,
        Type::Proj(s, ..) => bounded(s),
        _ => false,
    }
}

pub(super) fn has_proj(t: &Type) -> bool {
    matches!(t, Type::Proj(..)) || t.parts().any(has_proj)
}

/// Why the checker has to prove an obligation, for saying what's wrong if
/// it can't.
#[derive(Debug, Clone)]
pub(super) enum Why {
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
}

/// An obligation the checker has to prove once the types in it are known.
#[derive(Debug, Clone)]
pub(super) struct Obligation {
    pub(super) pred: Pred,
    pub(super) span: Span,
    pub(super) why: Why,
}

/// The method a call or path names.
pub(super) struct Found {
    /// What the call names: the method's symbol.
    pub(super) symbol: String,
    /// Its type, with its type parameters, all of them, replaced by `args`.
    pub(super) ty: Type,
    pub(super) args: Vec<Type>,
    /// The trait it's a method of, if it is, and the type that implements
    /// it, which must.
    pub(super) of_trait: Option<Pred>,
    /// Where it's declared, if anywhere.
    pub(super) def: Option<Span>,
    pub(super) receiver: bool,
}

impl Checker {
    /// Pass 0: the names of the traits, so that anything can mention any
    /// of them.
    pub(super) fn trait_names(&mut self, prog: &Program) -> Result<(), Error> {
        for t in &prog.traits {
            let name = &t.name;
            if self.traits.traits.contains_key(&name.name) {
                let message = match crate::traits::BUILTIN_TRAITS.contains(&name.name.as_str()) {
                    true => format!("`{}` is a built-in trait", name.name),
                    false => format!("trait `{}` is already defined", name.name),
                };
                return Err(Error::ty(name.span, message));
            }
            if prog.types.iter().any(|d| d.name().name == name.name) {
                return Err(Error::ty(
                    name.span,
                    format!("`{}` is already the name of a type", name.name),
                ));
            }
            let def = TraitDef {
                span: Some(name.span),
                supers: Vec::new(),
                assoc: Vec::new(),
                methods: Vec::new(),
            };
            self.traits.traits.insert(name.name.clone(), def);
            self.info.symbols.insert(name.id, name.name.clone());
        }
        // Each trait's supertraits and associated types, which a type
        // anywhere can mention, as `T::Item`.
        for t in &prog.traits {
            let mut supers = Vec::new();
            for s in &t.supers {
                let s = self.trait_named(s)?;
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            let mut assoc: Vec<AssocDecl> = Vec::new();
            for a in &t.assoc {
                if assoc.iter().any(|b| b.name == a.name.name) {
                    return Err(Error::ty(
                        a.name.span,
                        format!("associated type `{}` is declared twice", a.name.name),
                    ));
                }
                let mut bounds = Vec::new();
                for b in &a.bounds {
                    let b = self.trait_named(b)?;
                    if !bounds.contains(&b) {
                        bounds.push(b);
                    }
                }
                assoc.push(AssocDecl {
                    name: a.name.name.clone(),
                    bounds,
                    span: a.name.span,
                });
            }
            // An associated effect is an associated type of the kind of
            // effects: it's projected and normalised as one is.
            for e in &t.effects {
                if assoc.iter().any(|b| b.name == e.name.name) {
                    return Err(Error::ty(
                        e.name.span,
                        format!("associated effect `{}` is declared twice", e.name.name),
                    ));
                }
                assoc.push(AssocDecl {
                    name: e.name.name.clone(),
                    bounds: Vec::new(),
                    span: e.name.span,
                });
            }
            let def = self.traits.traits.get_mut(&t.name.name).expect("named");
            def.supers = supers;
            def.assoc = assoc;
        }
        Ok(())
    }

    /// The trait `name` written at `span` names, which must be one.
    pub(super) fn trait_named(&mut self, name: &Ident) -> Result<String, Error> {
        match self.traits.traits.get(&name.name) {
            Some(t) => {
                let kind = super::NoteKind::Trait(name.name.clone());
                let def = t.span;
                self.note(name.span, kind, None, def);
                Ok(name.name.clone())
            }
            None => Err(Error::ty(
                name.span,
                format!("unknown trait `{}`", name.name),
            )),
        }
    }

    /// The trait bounds in `g`, whose type parameters are `params` and
    /// lifetimes `lifetimes`, and what its equality bounds say: `T: Iter`
    /// and `(T::Item, int)` for `T: Iter<Item = int>`. Bounds on
    /// projections, like `T::Item: Discard` for `T: Iter<Item: Discard>`,
    /// only if `on_projections`: a function's, not an impl's or a struct's,
    /// whose bounds resolution would then have to prove of what `T::Item`
    /// stands for, which may be a bigger type than the impl's.
    pub(super) fn preds(
        &mut self,
        g: &Generics,
        params: &[String],
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
        params: &[String],
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
                // `F: Fn(int) -> int` says what `F`'s calls take and give,
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
                            "FnOnce".into(),
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
                                "say what calls of `{0}` take and give, as in `{0}: {tr}(int) -> \
                                 int`",
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
    pub(super) fn projection(
        &self,
        s: Type,
        tr: Option<&Ident>,
        name: &Ident,
        t: &TypeExpr,
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
        if !hole {
            let known = match &s {
                Type::Param(_) | Type::Proj(..) => traits_of(&s).contains(&tr),
                s => {
                    let local = &self.adts.proj.local;
                    self.collecting
                        || super::unify::has_vars(s)
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

    /// Pass 1: each trait's supertraits and the signatures of its methods.
    /// No trait can be its own supertrait, even through others: then
    /// implementing it would need it implemented already.
    pub(super) fn collect_traits(&mut self, prog: &Program) -> Result<(), Error> {
        for t in &prog.traits {
            if let Some(i) = t
                .supers
                .iter()
                .position(|s| self.traits.implied(&s.name).contains(&t.name.name))
            {
                return Err(Error::ty(
                    t.supers[i].span,
                    format!(
                        "`{}` can't be a supertrait of `{}`: it is `{}` itself, or needs it, so \
                         implementing either would need the other done first",
                        t.supers[i].name, t.name.name, t.name.name
                    ),
                ));
            }
        }
        for t in &prog.traits {
            let mut methods: Vec<MethodDef> = Vec::new();
            for m in &t.methods {
                let f = &m.decl;
                if methods.iter().any(|d| d.name == f.name.name) {
                    return Err(Error::ty(
                        f.name.span,
                        format!("method `{}` is declared twice", f.name.name),
                    ));
                }
                let this = Pred {
                    ty: Type::Param("Self".into()),
                    trait_name: t.name.name.clone(),
                };
                let mut scheme = self.method_scheme(f, &Outer::default(), Some(this), None)?;
                scheme.params.insert(0, "Self".into());
                scheme.preds.insert(
                    0,
                    Pred {
                        ty: Type::Param("Self".into()),
                        trait_name: t.name.name.clone(),
                    },
                );
                methods.push(MethodDef {
                    name: f.name.name.clone(),
                    symbol: trait_method_symbol(&t.name.name, &f.name.name),
                    scheme,
                    provided: m.provided,
                    receiver: f.receiver().is_some(),
                    span: Some(f.name.span),
                    effects_at: Some(super::effects::written_at(f)),
                });
            }
            self.traits
                .traits
                .get_mut(&t.name.name)
                .expect("named")
                .methods = methods;
        }
        Ok(())
    }

    /// The type of method `f`, in an impl with parameters, bounds and so
    /// on `outer`, for type `self_ty`, or in a trait if that's `None`,
    /// where `Self` is a type parameter. Generic over the impl's parameters
    /// and then its own. `this` is `Self: Trait`, which holds in it, if
    /// it's of a trait or an impl of one, so that `Self::Item` means
    /// something.
    pub(super) fn method_scheme(
        &mut self,
        f: &FnDecl,
        outer: &Outer,
        this: Option<Pred>,
        self_ty: Option<&Type>,
    ) -> Result<Scheme, Error> {
        let Outer {
            params: impl_params,
            lifetimes: impl_lifetimes,
            bounds: impl_bounds,
            preds: impl_preds,
            eqs: impl_eqs,
        } = outer;
        let own = generics::declare(&f.generics.types)?;
        if let Some(p) = f
            .generics
            .types
            .iter()
            .find(|p| impl_params.contains(&p.name))
        {
            return Err(Error::ty(
                p.span,
                format!(
                    "type parameter `{}` is the impl's already; give this one another name",
                    p.name
                ),
            ));
        }
        let declared = generics::declare_lifetimes(&f.generics.lifetimes)?;
        let mut params: Vec<String> = impl_params.to_vec();
        params.extend(own.iter().cloned());
        let mut lifetimes: Vec<String> = impl_lifetimes.to_vec();
        lifetimes.extend(declared.iter().cloned());
        let mut bounds = impl_bounds.to_vec();
        bounds.extend(generics::bounds(&f.generics, &lifetimes, &params)?);
        let mut preds = impl_preds.to_vec();
        let mut eqs = impl_eqs.to_vec();
        // `Self` resolves as a type parameter, and in an impl is then
        // replaced by the impl's type. A `where` clause can bound it, or
        // its associated types, which the trait's bound on it says.
        let mut with_self = params.clone();
        with_self.push("Self".into());
        self.env = preds.iter().cloned().chain(this.clone()).collect();
        self.adts.proj.local = eqs.clone();
        let (mut own_preds, mut own_eqs) = self.preds(&f.generics, &with_self, &lifetimes, true)?;
        if let Some(s) = self_ty {
            let s = Subst::new(&["Self".to_string()], std::slice::from_ref(s));
            for p in &mut own_preds {
                p.ty = self.adts.normalise(&s.apply(&p.ty));
            }
            for (l, r) in &mut own_eqs {
                (*l, *r) = (s.apply(l), s.apply(r));
            }
        }
        preds.extend(own_preds);
        eqs.extend(own_eqs);
        let opaques = this.is_none();
        self.env = preds.iter().cloned().chain(this).collect();
        self.adts.proj.local = eqs.clone();
        let (param_tys, ret, effects, lts) = self.signature(f, &with_self, &lifetimes, opaques)?;
        let mut ty = Type::func_with(param_tys, ret, effects);
        if let Some(s) = self_ty {
            let s = Subst::new(&["Self".to_string()], std::slice::from_ref(s));
            ty = self.adts.normalise(&s.apply(&ty));
            // What its opaque types' bounds say of `Self` is of the impl's
            // type too.
            for o in super::opaques_of(&ty) {
                for d in [
                    self.traits.opaques.get_mut(&o),
                    self.adts.proj.opaques.get_mut(&o),
                ]
                .into_iter()
                .flatten()
                {
                    for p in &mut d.preds {
                        p.ty = s.apply(&p.ty);
                    }
                    for (l, r) in &mut d.eqs {
                        (*l, *r) = (s.apply(l), s.apply(r));
                    }
                }
            }
        }
        self.env.clear();
        self.adts.proj.local.clear();
        Ok(Scheme {
            params,
            lifetimes: lts,
            bounds,
            preds,
            eqs,
            ty,
        })
    }

    /// Pass 2: every impl's parameters, type and bounds, and that it's for
    /// something it can be.
    pub(super) fn collect_impls(&mut self, prog: &Program) -> Result<(), Error> {
        self.collecting = true;
        let result = self.collect_impls_in(prog);
        self.collecting = false;
        result
    }

    fn collect_impls_in(&mut self, prog: &Program) -> Result<(), Error> {
        for d in &prog.impls {
            let params = generics::declare(&d.generics.types)?;
            let declared = generics::declare_lifetimes(&d.generics.lifetimes)?;
            let mut lts = Lifetimes::Params {
                declared: &declared,
                elided: Vec::new(),
            };
            if let Some(s) = mentions_self(&d.self_ty) {
                return Err(Error::ty(
                    s,
                    "`Self` is the type an impl is for, so it can't be used to say what that is",
                ));
            }
            if let Some(s) = proj_in(&d.self_ty) {
                return Err(Error::ty(
                    s,
                    "an impl can't be for a projection: what type it's for would depend on \
                     other impls, and two impls could then be for the same type",
                ));
            }
            let self_ty = self.resolve_with(&d.self_ty, &params, &mut lts)?;
            let mut lifetimes = declared.clone();
            if let Lifetimes::Params { elided, .. } = lts {
                lifetimes.extend(elided);
            }
            let bounds = generics::bounds(&d.generics, &lifetimes, &params)?;
            let (preds, eqs) = self.preds(&d.generics, &params, &lifetimes, false)?;
            let trait_name = match &d.trait_name {
                Some(t) => Some(self.trait_named(t)?),
                None => None,
            };
            let header = Span::new(d.span.start, d.self_ty.span.end);
            match (&trait_name, &self_ty) {
                // So all of a type's own methods are where its private
                // fields can be seen.
                (None, Type::Adt(symbol, ..)) => {
                    let def = self.types.iter().find(|t| t.symbol == *symbol);
                    if let Some(def) = def
                        && def.module != d.module
                    {
                        return Err(Error::ty(
                            d.self_ty.span,
                            format!(
                                "an inherent impl of `{self_ty}` must be in its module, {}",
                                self.modules.describe(def.module)
                            ),
                        ));
                    }
                }
                (None, t) => {
                    return Err(Error::ty(
                        d.self_ty.span,
                        format!(
                            "an inherent impl can only be for a struct or enum, not `{t}`; to \
                             give `{t}` methods, declare a trait and implement that"
                        ),
                    ));
                }
                (Some(tr), _) if crate::traits::FN_TRAITS.contains(&tr.as_str()) => {
                    return Err(Error::ty(
                        header,
                        format!(
                            "`{tr}` is built in: only functions and closures implement it, and \
                             references to them"
                        ),
                    ));
                }
                (Some(tr), t)
                    if crate::traits::BUILTIN_TRAITS.contains(&tr.as_str())
                        && !matches!(t, Type::Adt(..)) =>
                {
                    return Err(Error::ty(
                        d.self_ty.span,
                        format!(
                            "`{tr}` can only be implemented for a struct or enum: it's built \
                             into `{t}` already, or can't be"
                        ),
                    ));
                }
                _ => {}
            }
            if let Some(tr) = &trait_name {
                self.orphan(prog, d, tr, &self_ty)?;
            }
            // Each parameter must be decided by the type: otherwise nothing
            // could say what it is when the impl is used. An effect
            // parameter may be decided by an `Fn` bound instead, as the
            // least effects that make it hold (see `traits::impl_args`).
            for (p, decl) in params.iter().zip(&d.generics.types) {
                if is_effect_name(p) && in_fn_bound(&d.generics, p) {
                    continue;
                }
                if !mentions(&self_ty, p) {
                    return Err(Error::ty(
                        decl.span,
                        format!(
                            "type parameter `{p}` isn't in `{self_ty}`, so nothing could say \
                             what it is where the impl is used; remove it"
                        ),
                    ));
                }
            }
            if trait_name.is_some() {
                parametric(d, &self_ty, &params, &bounds)?;
            }
            let mut methods: Vec<(String, String)> = Vec::new();
            for f in &d.methods {
                if methods.iter().any(|(m, _)| *m == f.name.name) {
                    return Err(Error::ty(
                        f.name.span,
                        format!("method `{}` is defined twice", f.name.name),
                    ));
                }
                let symbol = self.fn_symbols[self.method_ids[&f.name.id].0].clone();
                methods.push((f.name.name.clone(), symbol));
            }
            if let Some(tr) = &trait_name {
                let def = &self.traits.traits[tr];
                for f in &d.methods {
                    if !def.methods.iter().any(|m| m.name == f.name.name) {
                        return Err(Error::ty(
                            f.name.span,
                            format!("`{}` isn't a method of trait `{tr}`", f.name.name),
                        ));
                    }
                }
                let missing: Vec<String> = def
                    .methods
                    .iter()
                    .filter(|m| !m.provided && !methods.iter().any(|(n, _)| *n == m.name))
                    .map(|m| format!("`{}`", m.name))
                    .collect();
                if !missing.is_empty() {
                    return Err(Error::ty(
                        header,
                        format!(
                            "this impl of `{tr}` for `{self_ty}` is missing {}: {}",
                            match missing.len() {
                                1 => "a method".to_string(),
                                n => plural(n, "method"),
                            },
                            missing.join(", ")
                        ),
                    ));
                }
            }
            let assoc = self.assoc_defs(
                d,
                trait_name.as_deref(),
                &self_ty,
                &params,
                &lifetimes,
                &preds,
                &eqs,
                header,
            )?;
            self.impl_ids.push(self.traits.impls.len());
            self.traits.impls.push(ImplDef {
                span: header,
                trait_name,
                params,
                lifetimes,
                bounds,
                self_ty,
                preds,
                eqs,
                assoc,
                methods,
                derived: false,
            });
        }
        Ok(())
    }

    /// What impl `d`, of trait `tr` (if it's of one) for `self_ty`, says
    /// the trait's associated types are, with the rules that keep working
    /// them out sound (see `traits::Projections`):
    ///
    /// - One for each of the trait's, and no others.
    /// - Every lifetime, as anywhere in an impl, is one of the impl's or
    ///   `'static`, and none is left out. So its regions are those of the
    ///   type the impl is for, or `'static`, and the borrow checker can
    ///   say `T: 'r` makes `T::Out: 'r`.
    /// - *Constrained*: each parameter it mentions is in `self_ty` outside
    ///   every function type, where matching the type decides it. Not in it
    ///   at all, `<S as Tr>::Out` could be a reference of any lifetime, a
    ///   new one each time it's mentioned. Only inside a function type, it
    ///   would be decided as a region only that function type means, as
    ///   finding an impl doesn't look at what function types are generic
    ///   over: `impl<A, B> Tr for fn(A) -> B` is found for `fn(&int) ->
    ///   &int`, with `B` the result's `&int`, of whatever lifetime a call
    ///   picks.
    /// - It *terminates*: each projection in it is on a type smaller than
    ///   `self_ty`, with no type parameter more often, and no projection in
    ///   that, or, in a blanket impl, on its parameter itself (see
    ///   `assoc_termination`). Then working out a projection ends.
    #[allow(clippy::too_many_arguments)]
    fn assoc_defs(
        &mut self,
        d: &ImplDecl,
        tr: Option<&str>,
        self_ty: &Type,
        params: &[String],
        lifetimes: &[String],
        preds: &[Pred],
        eqs: &[(Type, Type)],
        header: Span,
    ) -> Result<Vec<(String, Type)>, Error> {
        let Some(tr) = tr else {
            if let Some(e) = d.effects.first() {
                return Err(Error::ty(
                    e.name.span,
                    "only an impl of a trait can have associated effects",
                ));
            }
            return match d.assoc.first() {
                Some(a) => Err(Error::ty(
                    a.name.span,
                    "only an impl of a trait can have associated types",
                )),
                None => Ok(Vec::new()),
            };
        };
        let decls = self.traits.traits[tr].assoc.clone();
        let mut out: Vec<(String, Type)> = Vec::new();
        for a in &d.assoc {
            let n = &a.name.name;
            if !decls.iter().any(|x| x.name == *n) {
                return Err(Error::ty(
                    a.name.span,
                    format!("`{n}` isn't an associated type of trait `{tr}`"),
                ));
            }
            if out.iter().any(|(m, _)| m == n) {
                return Err(Error::ty(
                    a.name.span,
                    format!("associated type `{n}` is defined twice"),
                ));
            }
            let mut with_self = params.to_vec();
            with_self.push("Self".into());
            let mut lts = Lifetimes::Assoc {
                declared: lifetimes,
            };
            self.env = preds.to_vec();
            self.env.push(Pred {
                ty: Type::Param("Self".into()),
                trait_name: tr.to_string(),
            });
            self.adts.proj.local = eqs.to_vec();
            let resolved = self.resolve_with(&a.ty, &with_self, &mut lts);
            self.env.clear();
            self.adts.proj.local.clear();
            let t =
                Subst::new(&["Self".to_string()], std::slice::from_ref(self_ty)).apply(&resolved?);
            constrained(&t, self_ty, params, a.ty.span)?;
            terminates(&t, self_ty, params, a.ty.span)?;
            out.push((n.clone(), t));
        }
        for e in &d.effects {
            let n = &e.name.name;
            if !decls.iter().any(|x| x.name == *n) {
                return Err(Error::ty(
                    e.name.span,
                    format!(
                        "`{}` has no associated effect `{n}`",
                        super::source_name(tr)
                    ),
                ));
            }
            if out.iter().any(|(m, _)| m == n) {
                return Err(Error::ty(
                    e.name.span,
                    format!("associated effect `{n}` is defined twice"),
                ));
            }
            let mut with_self = params.to_vec();
            with_self.push("Self".into());
            let mut lts = Lifetimes::Assoc {
                declared: lifetimes,
            };
            self.env = preds.to_vec();
            self.env.push(Pred {
                ty: Type::Param("Self".into()),
                trait_name: tr.to_string(),
            });
            self.adts.proj.local = eqs.to_vec();
            let resolved = self.resolve_effects_with(&e.effects, &with_self, &mut lts);
            self.env.clear();
            self.adts.proj.local.clear();
            let t =
                Subst::new(&["Self".to_string()], std::slice::from_ref(self_ty)).apply(&resolved?);
            decided_effects(&t, self_ty, eqs, e.effects.span)?;
            terminates(&t, self_ty, params, e.effects.span)?;
            out.push((n.clone(), t));
        }
        let missing: Vec<&AssocDecl> = decls
            .iter()
            .filter(|x| !out.iter().any(|(n, _)| *n == x.name))
            .collect();
        let (effects, types): (Vec<&AssocDecl>, Vec<&AssocDecl>) =
            missing.into_iter().partition(|x| is_effect_name(&x.name));
        for (missing, what) in [(types, "associated type"), (effects, "associated effect")] {
            if missing.is_empty() {
                continue;
            }
            let names: Vec<String> = missing.iter().map(|x| format!("`{}`", x.name)).collect();
            return Err(Error::ty(
                header,
                format!(
                    "this impl of `{}` for `{self_ty}` is missing {}: {}",
                    super::source_name(tr),
                    match names.len() {
                        1 => format!("an {what}"),
                        n => plural(n, what),
                    },
                    names.join(", ")
                ),
            ));
        }
        Ok(out)
    }

    /// Once every impl is collected, what each projection stands for is
    /// known: the types already resolved, of fields and of trait methods,
    /// get theirs worked out.
    pub(super) fn projections_known(&mut self) {
        let local = std::mem::take(&mut self.adts.proj.local);
        self.adts.proj = crate::traits::Projections::new(&self.traits);
        let proj = self.adts.proj.clone();
        for def in self.adts.values_mut() {
            let mut p = proj.clone();
            p.local = def.eqs.clone();
            match &mut def.kind {
                AdtKind::Struct(fields) => {
                    for (_, t) in fields {
                        *t = p.normalise(t);
                    }
                }
                AdtKind::Enum(variants) => {
                    for t in variants.iter_mut().flat_map(|(_, ts)| ts) {
                        *t = p.normalise(t);
                    }
                }
            }
        }
        for def in self.traits.traits.values_mut() {
            for m in &mut def.methods {
                m.scheme.ty = proj.normalise(&m.scheme.ty);
            }
        }
        self.adts.proj.local = local;
    }

    /// The impl of each `#[derive(..)]`. What it needs of the type's
    /// parameters is worked out from its fields: exactly the bounds that
    /// make every field implement the trait. `#[derive(Copy)] struct
    /// W<T> { f: fn(T) -> int }` is `Copy` for every `T`, and `struct P<A,
    /// B> { a: Option<A>, b: &B }` only needs `A: Copy`. A bound can be of
    /// another trait: with `impl<T: Show> Eq for Q<T>`, a derived `Eq` of
    /// `struct W<T> { q: Q<T> }` needs `T: Show`, and `T: Eq` wouldn't do,
    /// as it doesn't make `Q<T>: Eq`.
    ///
    /// A field can be of a type whose impl is derived too, maybe the type
    /// itself, through a reference: `struct Node<'a, T> { v: T, next:
    /// Option<&'a Node<'a, T>> }`. So the bounds are worked out together,
    /// starting from none and adding what's missing until nothing is: the
    /// least *fixed point*. That ends, as each round adds a bound or stops,
    /// and there are only so many parameters and traits.
    ///
    /// A derived `Copy` needs the type to be `Discard` too, as `trait Copy:
    /// Discard`, and it's never assumed: it says so with a fix.
    ///
    /// What a public struct with private fields derives says what it needs
    /// in a `where`, or needs nothing, if it has none (lemma M5 of
    /// docs/06d-modules.md): those bounds are part of its interface, which
    /// its private fields mustn't change. Whatever the fields need must
    /// follow from what's written, which may say more, to leave room.
    pub(super) fn derive(&mut self, prog: &Program) -> Result<(), Error> {
        let decls = prog.all_types();
        // (impl index, type, the derive, trait)
        let mut derived: Vec<(usize, usize, Span, &'static str)> = Vec::new();
        for (i, (d, _)) in decls.iter().enumerate() {
            for tr in ["Copy", "Discard", "Eq"] {
                let Some(at) = d.derive_of(tr) else {
                    continue;
                };
                let symbol = &self.types[i].symbol;
                let def = &self.adts[symbol];
                derived.push((self.traits.impls.len(), i, at.span, tr));
                self.traits.impls.push(ImplDef {
                    span: at.span,
                    trait_name: Some(tr.to_string()),
                    params: def.params.clone(),
                    lifetimes: def.lifetimes.clone(),
                    bounds: def.bounds.clone(),
                    self_ty: def.generic(symbol),
                    preds: Vec::new(),
                    eqs: def.eqs.clone(),
                    assoc: Vec::new(),
                    methods: Vec::new(),
                    derived: true,
                });
            }
        }
        // The bounds written down, by impl.
        let mut written: HashMap<usize, Option<&Where>> = HashMap::new();
        for &(k, i, _, tr) in &derived {
            let (d, _) = decls[i];
            let boundary = match d {
                TypeDecl::Struct(s) => {
                    s.public.is_some() && s.fields.iter().any(|f| f.public.is_none())
                }
                TypeDecl::Enum(_) => false,
            };
            let w = d.derive_where(tr);
            if w.is_none() && !boundary {
                continue;
            }
            if let Some(w) = w {
                let def = &self.adts[&self.types[i].symbol];
                let (params, lifetimes) = (def.params.clone(), def.lifetimes.clone());
                if let Some((p, _)) = w.bounds.iter().find(|(p, _)| !params.contains(&p.name)) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{}` isn't a type parameter of `{}`", p.name, d.name().name),
                    ));
                }
                let g = Generics {
                    bounds: w.bounds.clone(),
                    ..Generics::default()
                };
                let (preds, eqs) = self.preds(&g, &params, &lifetimes, false)?;
                self.traits.impls[k].preds = preds;
                self.traits.impls[k].eqs.extend(eqs);
            }
            written.insert(k, w);
        }
        loop {
            let mut changed = false;
            for &(k, i, _, tr) in &derived {
                let (d, _) = decls[i];
                let symbol = &self.types[i].symbol;
                let def = &self.adts[symbol];
                let mut needed: Vec<Pred> = Vec::new();
                for (what, t) in fields(&def.kind) {
                    if let Err(bad) = self.needs(&t, tr, &def.params, &mut needed) {
                        // A derived impl can't have a bound on a projection
                        // (see `preds`): only the trait's own will do.
                        if let Type::Proj(_, of, a) = &bad.ty {
                            let p = &bad.ty;
                            let held = match bad.ty == t {
                                true => format!("holds a `{t}`"),
                                false => format!("holds a `{t}`, which has a `{p}` in it"),
                            };
                            return Err(Error::ty(
                                derived.iter().find(|x| x.0 == k).expect("it").2,
                                format!(
                                    "`{}` can't be `{tr}`: its {what} {held}, and `{of}` \
                                     doesn't say that's `{tr}`; declare it as `type {a}: {tr};`",
                                    d.name().name
                                ),
                            ));
                        }
                        let why = match (&bad.ty, tr) {
                            (Type::Fn(_), "Eq") => " (functions can't be compared)".to_string(),
                            (b, _) if *b == t => String::new(),
                            (b, _) => format!(" (it needs `{b}: {tr}`, and that doesn't hold)"),
                        };
                        return Err(Error::ty(
                            derived.iter().find(|x| x.0 == k).expect("it").2,
                            format!(
                                "`{}` can't be `{tr}`: its {what} holds a `{t}`, which isn't{why}",
                                d.name().name
                            ),
                        ));
                    }
                }
                // In a fixed order, so that a round that adds none changes
                // nothing.
                let position = |p: &Pred| {
                    def.params
                        .iter()
                        .position(|q| p.ty == Type::Param(q.clone()))
                };
                needed.sort_by(|a, b| {
                    (position(a), &a.trait_name).cmp(&(position(b), &b.trait_name))
                });
                let preds = needed;
                if written.contains_key(&k) {
                    continue;
                }
                if preds != self.traits.impls[k].preds {
                    self.traits.impls[k].preds = preds;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // What's written must give what the fields need.
        for &(k, i, span, tr) in &derived {
            let Some(&w) = written.get(&k) else {
                continue;
            };
            let (d, _) = decls[i];
            let def = &self.adts[&self.types[i].symbol];
            let mut needed: Vec<Pred> = Vec::new();
            for (_, t) in fields(&def.kind) {
                // What can't hold at all was reported above.
                let _ = self.needs(&t, tr, &def.params, &mut needed);
            }
            let imp = &self.traits.impls[k];
            let missing: Vec<&Pred> = needed
                .iter()
                .filter(|p| {
                    self.traits
                        .implements(&p.ty, &p.trait_name, &imp.preds, &imp.eqs)
                        .is_err()
                })
                .collect();
            let Some(first) = missing.first() else {
                continue;
            };
            let name = &d.name().name;
            let bounds: Vec<String> = needed.iter().map(|p| format!("{p}")).collect();
            let at = d.derive_of(tr).expect("derived");
            return Err(match w {
                Some(w) => Error::ty(
                    w.span,
                    format!("`{name}`'s `{tr}` needs `{first}`, which its `where` doesn't say"),
                ),
                None => Error::ty(
                    span,
                    format!(
                        "`{name}` is public with private fields, so its `{tr}` must say what it \
                         needs: `where {}`; what its private fields need can't be part of its \
                         interface",
                        bounds.join(", ")
                    ),
                )
                .with_fix(Fix::new(
                    "say what it needs",
                    vec![Edit::insert(
                        at.span.end,
                        format!(" where {}", bounds.join(", ")),
                    )],
                )),
            });
        }
        for &(k, i, span, tr) in &derived {
            let def = &self.traits.impls[k];
            if tr != "Copy"
                || self
                    .traits
                    .implements(&def.self_ty, "Discard", &def.preds, &def.eqs)
                    .is_ok()
            {
                continue;
            }
            let (d, _) = decls[i];
            let name = &d.name().name;
            let e = Error::ty(
                span,
                format!(
                    "`{name}` can't be `Copy` unless it's `Discard`: `trait Copy: Discard`, as a \
                     value that's copied can't be used up exactly once"
                ),
            );
            let copy = d.derive_of("Copy").expect("derived");
            return Err(match d.derive_of("Discard") {
                Some(_) => e,
                None => e.with_fix(Fix::new(
                    "derive `Discard` too",
                    vec![Edit::insert(copy.span.start, "Discard, ")],
                )),
            });
        }
        Ok(())
    }

    /// What `t` needs of `params` to implement `tr`, by the impls found so
    /// far: bounds on them, added to `out`, each of whatever trait it is.
    /// Or the obligation that fails whatever they implement.
    fn needs(
        &self,
        t: &Type,
        tr: &str,
        params: &[String],
        out: &mut Vec<Pred>,
    ) -> Result<(), Pred> {
        match t {
            Type::Param(p) if params.contains(p) => {
                let pred = Pred {
                    ty: t.clone(),
                    trait_name: tr.to_string(),
                };
                if !out.contains(&pred) {
                    out.push(pred);
                }
                return Ok(());
            }
            Type::Ref(_, _, inner) | Type::Box(inner) if tr == "Eq" => {
                return self.needs(inner, tr, params, out);
            }
            Type::Box(inner) if tr == "Discard" => return self.needs(inner, tr, params, out),
            Type::Adt(..) => {}
            t => {
                return self
                    .traits
                    .implements(t, tr, &[], &[])
                    .map(|_| ())
                    .map_err(|_| Pred {
                        ty: t.clone(),
                        trait_name: tr.to_string(),
                    });
            }
        }
        let fail = || Pred {
            ty: t.clone(),
            trait_name: tr.to_string(),
        };
        let (i, args) = self.traits.select(t, Some(tr), &[]).ok_or_else(fail)?;
        let d = &self.traits.impls[i];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            self.needs(&s.apply(&p.ty), &p.trait_name, params, out)?;
        }
        Ok(())
    }

    /// Pass 4, once every impl and method signature is known: the rules
    /// for impls. Each impl of a trait has the trait's methods at the
    /// trait's types, and the trait's supertraits; an impl of `Copy` or
    /// `Discard` is for a type whose fields all are; no two impls overlap;
    /// and proving
    /// that a type implements a trait always ends.
    pub(super) fn check_impls(&mut self, prog: &Program) -> Result<(), Error> {
        for (k, d) in prog.impls.iter().enumerate() {
            let def = self.traits.impls[self.impl_ids[k]].clone();
            self.env = def.preds.clone();
            self.well_formed_now(&def.self_ty, d.self_ty.span)?;
            let Some(tr) = &def.trait_name else {
                continue;
            };
            for f in &d.methods {
                let id = self.method_ids[&f.name.id];
                let got = self.fns[id.0].scheme.clone();
                let want = self
                    .traits
                    .method(tr, &f.name.name)
                    .expect("checked")
                    .clone();
                self.same_signature(f, &got, &want, &def)?;
            }
            for s in self.traits.traits[tr].supers.clone() {
                if self
                    .traits
                    .implements(&def.self_ty, &s, &def.preds, &def.eqs)
                    .is_err()
                {
                    return Err(Error::ty(
                        def.span,
                        format!(
                            "`{}` doesn't implement `{s}`, which `{tr}` needs, as it's \
                             `trait {tr}: {}`",
                            def.self_ty,
                            self.traits.traits[tr].supers.join(" + ")
                        ),
                    ));
                }
            }
            self.assoc_bounds(&def, d)?;
            if tr == "Copy" || tr == "Discard" {
                let Type::Adt(symbol, ..) = &def.self_ty else {
                    unreachable!("checked")
                };
                let adt = &self.adts[symbol];
                let s = Subst::new(&adt.params, &args_of(&def.self_ty));
                for (what, t) in fields(&adt.kind) {
                    let t = self.adts.normalise(&s.apply(&t));
                    if self
                        .traits
                        .implements(&t, tr, &def.preds, &def.eqs)
                        .is_err()
                    {
                        return Err(Error::ty(
                            def.span,
                            format!(
                                "`{}` can't be `{tr}`: its {what} holds a `{t}`, which isn't",
                                def.self_ty
                            ),
                        ));
                    }
                }
            }
        }
        self.env.clear();
        self.adts.proj.local.clear();
        self.coherence()?;
        self.termination()?;
        self.assoc_termination()?;
        self.assoc_sizes()?;
        self.assoc_regular()
    }

    /// Monomorphisation makes a copy of every struct and enum type a copy
    /// mentions, even through a box, and a field's projection is what an
    /// impl says it is, which can mention more: `struct S<T: Tr> { b:
    /// Box<T::Out> }` with `type Out = S<W<W<U>>>` for `W<U>` makes
    /// `S<W<int>>` need `S<W<W<int>>>`, and so on, with no end. So that's
    /// polymorphic recursion too (`generics::polymorphic_recursion`), with
    /// each impl's associated type a node of its own, generic over the
    /// impl's parameters, which a projection in a field gets to the way a
    /// method call gets to an impl's method (`dispatch`), and which uses
    /// the types and projections in what it says.
    fn assoc_regular(&self) -> Result<(), Error> {
        // A name for each associated type of each impl.
        let nodes: Vec<(usize, String, String)> = self
            .traits
            .impls
            .iter()
            .enumerate()
            .flat_map(|(k, d)| {
                let tr = d.trait_name.clone().unwrap_or_default();
                d.assoc
                    .iter()
                    .map(move |(n, _)| (k, n.clone(), format!("<{} as {tr}>::{n}", d.self_ty)))
            })
            .collect();
        let mut uses: Vec<generics::Use> = Vec::new();
        // To each impl a projection on `s` in `from` may be worked out by.
        let through = |from_params: &[String], s: &Type, tr: &str, name: &str, span: Span| {
            let mut out = Vec::new();
            let mut hidden: Vec<Hidden> = Vec::new();
            let s = &hide_projections(s, from_params, &mut hidden);
            for (k, n, node) in &nodes {
                let d = &self.traits.impls[*k];
                if n != name || d.trait_name.as_deref() != Some(tr) {
                    continue;
                }
                let renamed: Vec<Type> = d
                    .params
                    .iter()
                    .map(|p| Type::Param(format!("{p}'")))
                    .collect();
                let header = Subst::new(&d.params, &renamed).apply(&d.self_ty);
                let mut vars: Vec<String> = d.params.iter().map(|p| format!("{p}'")).collect();
                vars.extend(from_params.iter().cloned());
                vars.extend(hidden.iter().map(|h| h.0.clone()));
                if !overlap(&header, s, &vars) {
                    continue;
                }
                let mut found: Vec<Option<Type>> = vec![None; d.params.len()];
                let mut inside = Vec::new();
                split(
                    &d.self_ty,
                    s,
                    &d.params,
                    (from_params, &hidden),
                    &mut found,
                    &mut inside,
                );
                let args = found
                    .into_iter()
                    .map(|t| shown_projections(&t.unwrap_or(Type::Unit), &hidden))
                    .collect();
                out.push((node.as_str(), args, inside, span));
            }
            out
        };
        let mut edges: Vec<Edge> = Vec::new();
        let mut names: Vec<&String> = self.adts.keys().collect();
        names.sort();
        for a in &names {
            let def = &self.adts[*a];
            // Its name, where it's declared.
            let at = self.defs.types.get(*a).copied().unwrap_or(Span::new(0, 0));
            for (_, f) in fields(&def.kind) {
                super::each_adt(&f, &mut |to, args| {
                    edges.push((
                        (*a).clone(),
                        self.adt_key(to),
                        args.to_vec(),
                        Vec::new(),
                        at,
                    ));
                });
                each_proj(&f, &mut |s, tr| {
                    if let Some(name) = proj_name(&f, s, tr) {
                        for (to, args, inside, span) in through(&def.params, s, tr, &name, at) {
                            edges.push(((*a).clone(), to, args, inside, span));
                        }
                    }
                });
            }
        }
        for (k, n, node) in &nodes {
            let d = &self.traits.impls[*k];
            let (_, t) = d.assoc.iter().find(|(m, _)| m == n).expect("it");
            super::each_adt(t, &mut |to, args| {
                edges.push((
                    node.clone(),
                    self.adt_key(to),
                    args.to_vec(),
                    Vec::new(),
                    d.span,
                ));
            });
            each_proj(t, &mut |s, tr| {
                if let Some(name) = proj_name(t, s, tr) {
                    for (to, args, inside, span) in through(&d.params, s, tr, &name, d.span) {
                        edges.push((node.clone(), to, args, inside, span));
                    }
                }
            });
        }
        if nodes.is_empty() {
            return Ok(());
        }
        uses.extend(
            edges
                .iter()
                .map(|(from, to, args, inside, span)| generics::Use {
                    from,
                    to,
                    args: args.clone(),
                    inside: inside.clone(),
                    span: *span,
                }),
        );
        uses.sort_by_key(|u| u.span.start);
        let mut params: HashMap<&str, &[String]> = self
            .adts
            .iter()
            .map(|(name, def)| (name.as_str(), def.params.as_slice()))
            .collect();
        for (k, _, node) in &nodes {
            params.insert(node, &self.traits.impls[*k].params);
        }
        generics::polymorphic_recursion(&uses, &params)
    }

    /// The name a struct or enum's symbol is a node by, in `assoc_regular`.
    fn adt_key<'a>(&'a self, symbol: &str) -> &'a str {
        self.adts.get_key_value(symbol).expect("a type").0
    }

    /// What impl `def` (written as `d`) says its trait's associated types
    /// are must make sense where it's used: what's in them implements what
    /// it must, projections in them are worked out, and they implement what
    /// the trait declares of them, which generic code assumes.
    fn assoc_bounds(&mut self, def: &ImplDef, d: &ImplDecl) -> Result<(), Error> {
        let Some(tr) = &def.trait_name else {
            return Ok(());
        };
        self.env = def.preds.clone();
        self.adts.proj.local = def.eqs.clone();
        for (name, rhs) in &def.assoc {
            let span = d
                .assoc
                .iter()
                .find(|a| a.name.name == *name)
                .map_or(def.span, |a| a.ty.span);
            let t = self.adts.normalise(rhs);
            if let Some(p) = rigid_on_type(&t) {
                let Type::Proj(s, of, _) = &p else {
                    unreachable!("a projection")
                };
                return Err(Error::ty(
                    span,
                    format!("`{s}` doesn't implement `{of}`, so `{p}` means nothing"),
                ));
            }
            self.well_formed_now(&t, span)?;
            let decl = self.traits.traits[tr]
                .assoc
                .iter()
                .find(|a| a.name == *name)
                .expect("checked")
                .clone();
            for b in &decl.bounds {
                let local = &self.adts.proj.local;
                if let Err(bad) = self.traits.implements(&t, b, &self.env, local) {
                    let needs = match bad.ty == t {
                        true => String::new(),
                        false => format!(" (that needs `{bad}`)"),
                    };
                    return Err(Error::ty(
                        span,
                        format!(
                            "`{t}` doesn't implement `{b}`, which `{tr}` needs of `{name}`, as \
                             it declares `type {name}: {}`{needs}",
                            decl.bounds.join(" + ")
                        ),
                    ));
                }
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
    fn assoc_termination(&self) -> Result<(), Error> {
        let mut edges: Vec<(&str, &str, &ImplDef)> = Vec::new();
        for d in &self.traits.impls {
            let (Some(tr), Type::Param(_)) = (&d.trait_name, &d.self_ty) else {
                continue;
            };
            for (_, t) in &d.assoc {
                each_proj(t, &mut |s, to| {
                    if *s == d.self_ty {
                        edges.push((tr, to, d));
                    }
                });
            }
        }
        for &(from, to, d) in &edges {
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
    /// Tr> { x: T::Out }` with `impl Tr for int { type Out = S<int>; }` has
    /// an `S<int>` hold an `S<int>`, so it would have infinite size, which
    /// neither says alone (`collect_adts` checks each struct and enum on
    /// its own). Every such cycle goes through some impl's associated type,
    /// so from each, what it holds by value is followed, through fields and
    /// the impls projections are worked out by: getting back to an impl
    /// already on the way is a type that holds itself, or holds a bigger
    /// one of itself, again and again, which is as infinite.
    fn assoc_sizes(&self) -> Result<(), Error> {
        for (k, d) in self.traits.impls.iter().enumerate() {
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

    /// The orphan rule (lemma C2 of docs/06e-core.md): an impl of trait `tr`
    /// for `self_ty`, `d`, must be in the package of the trait, or of the
    /// type's head, its outermost constructor. Built-in traits and types
    /// are the core library's. So the core library's impls are all it has
    /// for its own traits and types, whatever program it's in, and no
    /// program can add one that overlaps them, or give a different one to
    /// another program: coherence holds for each package on its own.
    /// Looking only at the head is the strictest form of the rule, and can
    /// be relaxed.
    fn orphan(&self, prog: &Program, d: &ImplDecl, tr: &str, self_ty: &Type) -> Result<(), Error> {
        if self.modules.in_core(d.module) {
            return Ok(());
        }
        let trait_local = prog
            .traits
            .iter()
            .find(|t| t.name.name == tr)
            .is_some_and(|t| !self.modules.in_core(t.module));
        let head_local = match self_ty {
            Type::Adt(symbol, ..) => self
                .types
                .iter()
                .find(|t| t.symbol == *symbol)
                .is_none_or(|t| !self.modules.in_core(t.module)),
            _ => false,
        };
        if trait_local || head_local {
            return Ok(());
        }
        let head = match self_ty {
            Type::Adt(symbol, ..) => symbol.rsplit("::").next().unwrap_or(symbol).to_string(),
            t => t.to_string(),
        };
        let tr = tr.rsplit("::").next().unwrap_or(tr);
        Err(Error::ty(
            Span::new(d.span.start, d.self_ty.span.end),
            format!(
                "`{tr}` and `{head}` are both `core`'s, so only `core` can implement one for the \
                 other; to implement it for a type of your own, wrap it in a struct"
            ),
        ))
    }

    /// Checks that method `f`, of type `got`, of impl `d`, is method
    /// `want` of its trait, at the impl's type. Up to the names of their
    /// own type parameters and lifetimes: the impl's method can't take less
    /// or more than the trait says, or need more of its type parameters,
    /// or callers going by the trait's signature would be wrong.
    fn same_signature(
        &self,
        f: &FnDecl,
        got: &Scheme,
        want: &MethodDef,
        d: &ImplDef,
    ) -> Result<(), Error> {
        let tr = d.trait_name.as_deref().expect("a trait's impl");
        let mismatch = |what: String| {
            let decl = want.span;
            let e = Error::ty(
                f.name.span,
                format!(
                    "method `{}` doesn't match its declaration in trait `{tr}`: {what}",
                    f.name.name
                ),
            );
            match decl {
                Some(span) => e.with_note(span, "declared here".to_string()),
                None => e,
            }
        };
        let own = &got.params[d.params.len()..];
        let theirs = &want.scheme.params[1..];
        if own.len() != theirs.len() {
            return Err(mismatch(format!(
                "it has {} but should have {}",
                plural(own.len(), "type parameter"),
                theirs.len()
            )));
        }
        // The trait's, at `Self` = the impl's type and with this one's
        // type parameter names.
        let mut args = vec![d.self_ty.clone()];
        args.extend(own.iter().map(|p| crate::types::param(p)));
        let s = Subst::new(&want.scheme.params, &args);
        // `Self::Item` in the trait's is what this impl says it is, with
        // what its bounds say, which decide its effect parameters.
        let mut proj = self.adts.proj.clone();
        proj.local = d.eqs.clone();
        let wanted = proj.normalise(&s.apply(&want.scheme.ty));
        let (Type::Fn(g), Type::Fn(w)) = (&got.ty, &wanted) else {
            unreachable!("methods have function types")
        };
        if g.params.len() != w.params.len() {
            return Err(mismatch(format!(
                "it takes {} but should take {}",
                plural(g.params.len(), "parameter"),
                w.params.len()
            )));
        }
        let fixed = &d.lifetimes;
        let mut a = Canon::new(fixed);
        let mut b = Canon::new(fixed);
        for (i, (x, y)) in g.params.iter().zip(&w.params).enumerate() {
            if a.ty(x) != b.ty(y) {
                let which = match (i, want.receiver) {
                    (0, true) => "its receiver".to_string(),
                    (i, true) => format!("parameter {i}"),
                    (i, false) => format!("parameter {}", i + 1),
                };
                return Err(mismatch(format!("{which} should be `{y}`, not `{x}`")));
            }
        }
        if a.ty(&g.ret) != b.ty(&w.ret) {
            return Err(mismatch(format!(
                "it should return `{}`, not `{}`",
                w.ret, g.ret
            )));
        }
        // Its effects, at most the trait's: callers going by the trait's
        // signature may count on its calls having no others.
        let more = Type::effects(
            (g.effects.effect_items().iter())
                .filter(|e| !w.effects.effect_items().contains(e))
                .cloned(),
        );
        if !more.is_no_effects() {
            let e = Error::ty(
                f.name.span,
                format!(
                    "`{0}` {1}, but `{2}`'s `{0}` can't, so its impls can't either",
                    f.name.name,
                    super::effects::does(&more),
                    super::source_name(tr),
                ),
            );
            // Unless what's more is the impl's own, the trait's can say it;
            // or rather the impl's associated effect, if the trait says one.
            let own = more
                .effect_items()
                .iter()
                .all(|e| matches!(e, Type::Alloc | Type::Panic))
                && !(want.effects_at.iter())
                    .flat_map(|(_, w)| w)
                    .any(|w| w.starts_with("Self::"));
            return Err(match (&want.effects_at, own) {
                (Some((span, written)), true) => {
                    let mut items = written.clone();
                    items.extend(more.effect_items().iter().map(Type::to_string));
                    let text = super::effects::set_text(&items);
                    e.with_fix(Fix::new(
                        format!("say `with {text}` in `{}`", super::source_name(tr)),
                        vec![Edit {
                            span: *span,
                            text: format!(" with {text}"),
                        }],
                    ))
                }
                _ => e,
            });
        }
        // Its own lifetime bounds, the same as the trait's.
        let show = |bounds: &[Bound], c: &mut Canon| -> Vec<String> {
            let mut v: Vec<String> = bounds
                .iter()
                .map(|b| match b {
                    Bound::Region(x, y) => format!("{}: {}", c.region(x), c.region(y)),
                    Bound::Param(p, y) => {
                        let p = s.apply(&Type::Param(p.clone()));
                        format!("{p}: {}", c.region(y))
                    }
                    Bound::Proj(p, y) => format!("{}: {}", s.apply(p), c.region(y)),
                })
                .collect();
            v.sort();
            v
        };
        let gb = show(&got.bounds[d.bounds.len()..], &mut a);
        let wb = show(&want.scheme.bounds, &mut b);
        if gb != wb {
            return Err(mismatch(format!(
                "its lifetime bounds should be {}",
                match wb.is_empty() {
                    true => "none".to_string(),
                    false => format!("`{}`", wb.join(", ")),
                }
            )));
        }
        // Its own trait bounds, which can't be more than the trait's.
        let mut gp: Vec<String> = got.preds[d.preds.len()..]
            .iter()
            .map(|p| p.to_string())
            .collect();
        let mut wp: Vec<String> = want.scheme.preds[1..]
            .iter()
            .map(|p| Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            })
            .map(|p| p.to_string())
            .collect();
        gp.sort();
        wp.sort();
        if gp != wp {
            return Err(mismatch(format!(
                "its bounds should be {}",
                match wp.is_empty() {
                    true => "none".to_string(),
                    false => format!("`{}`", wp.join(", ")),
                }
            )));
        }
        Ok(())
    }

    /// No two impls of a trait can be for the same type, whatever their
    /// type parameters are: then which one a use means would be a choice.
    /// Their bounds don't count. `impl<T: Show> Print for T` and `impl
    /// Print for int` overlap even if `int` isn't `Show`, because someone
    /// might make it so, and then an impl far away would change meaning.
    ///
    /// Inherent impls can overlap, as long as they don't both define a
    /// method of the same name.
    fn coherence(&self) -> Result<(), Error> {
        let impls = &self.traits.impls;
        for (i, a) in impls.iter().enumerate() {
            for b in &impls[..i] {
                if a.trait_name != b.trait_name {
                    continue;
                }
                // `b`'s parameters, renamed apart from `a`'s.
                let renamed: Vec<Type> = b
                    .params
                    .iter()
                    .map(|p| Type::Param(format!("{p}'")))
                    .collect();
                let b_ty = Subst::new(&b.params, &renamed).apply(&b.self_ty);
                let mut params = a.params.clone();
                params.extend(b.params.iter().map(|p| format!("{p}'")));
                if !overlap(&a.self_ty, &b_ty, &params) {
                    continue;
                }
                let Some(tr) = &a.trait_name else {
                    if let Some((m, _)) = a
                        .methods
                        .iter()
                        .find(|(m, _)| b.methods.iter().any(|(n, _)| n == m))
                    {
                        return Err(Error::ty(
                            a.span,
                            format!(
                                "method `{m}` is defined for `{}` here, and for `{}` in another \
                                 impl, and those can be the same type",
                                a.self_ty, b.self_ty
                            ),
                        )
                        .with_note(b.span, "the other impl".to_string()));
                    }
                    continue;
                };
                // Point at the one written, rather than a derive.
                let (a, b) = match a.derived && !b.derived {
                    true => (b, a),
                    false => (a, b),
                };
                let what = |d: &ImplDef| match d.derived {
                    true => format!("the `#[derive({tr})]`"),
                    false => format!("`impl {}`", d.header()),
                };
                let same = a.self_ty.erased() == b.self_ty.erased();
                let message = match same {
                    true => format!(
                        "`{}` implements `{tr}` twice: here and in {}",
                        a.self_ty,
                        what(b)
                    ),
                    false => format!(
                        "this impl of `{tr}` overlaps {}: a type can be both `{}` and `{}`, and \
                         then which impl it uses would be a choice",
                        what(b),
                        a.self_ty,
                        b.self_ty
                    ),
                };
                let note = "the other impl".to_string();
                return Err(Error::ty(a.span, message).with_note(b.span, note));
            }
        }
        Ok(())
    }

    /// Proving `T: A` by an impl needs what the impl needs. For each
    /// impl, those are on its type parameters, each a part of the type it's
    /// for (each is in it, checked in `collect_impls`). So they're about
    /// smaller types, and the search ends: the *Paterson conditions*,
    /// which a bound on a type parameter meets whenever the impl is for a
    /// type that's more than that parameter.
    ///
    /// An impl for a bare parameter, a *blanket impl* like `impl<T: A> B
    /// for T`, needs `T: A` to prove `T: B`: the same type. That still ends
    /// unless there's a way back from `A` to `B`, like `impl<T: B> A for T`,
    /// so the blanket impls between traits can't form a cycle.
    fn termination(&self) -> Result<(), Error> {
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
        for &(from, to, d) in &edges {
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
    pub(super) fn oblige(&mut self, pred: Pred, span: Span, why: Why) -> Result<(), Error> {
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
        match later || super::unify::has_vars(&o.pred.ty) {
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
    pub(super) fn prove(&self, o: &Obligation) -> Result<(), Error> {
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
                    super::source_name(&t.to_string())
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
    pub(super) fn prove_all(&mut self) -> Result<(), Error> {
        for mut o in std::mem::take(&mut self.obligations) {
            o.pred.ty = self.decided_regions(&self.solved(&o.pred.ty));
            // If it's still not fully known, that's reported later.
            if !super::unify::has_vars(&o.pred.ty) {
                self.prove(&o)?;
            }
        }
        Ok(())
    }

    /// Requires what struct or enum types in `t` need of their type
    /// arguments, by their bounds: `S<int>` for `struct S<T: Show>` needs
    /// `int: Show`.
    pub(super) fn well_formed(&mut self, t: &Type, span: Span) -> Result<(), Error> {
        let mut preds = Vec::new();
        let mut eqs = Vec::new();
        super::each_adt(t, &mut |name, args| {
            if let Some(def) = self.adts.get(name) {
                let s = Subst::new(&def.params, args);
                for p in &def.preds {
                    let why = Why::Bound(super::source_name(name), p.ty.to_string());
                    let pred = Pred {
                        ty: s.apply(&p.ty),
                        trait_name: p.trait_name.clone(),
                    };
                    preds.push((pred, why));
                }
                for (lhs, rhs) in &def.eqs {
                    eqs.push((s.apply(lhs), s.apply(rhs), super::source_name(name)));
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
    pub(super) fn well_formed_now(&mut self, t: &Type, span: Span) -> Result<(), Error> {
        self.well_formed(t, span)
    }

    /// The method `name` of values of type `ty`, which must be known, for
    /// `x.name(..)`. See `lookup_method`.
    pub(super) fn method_of(
        &mut self,
        ty: &Type,
        name: &Ident,
        targs: &[TypeExpr],
    ) -> Result<Found, Error> {
        self.lookup_method(ty, None, name, targs, "value")
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
    pub(super) fn lookup_method(
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
                    out.push("Eq".into());
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
        let own = &def.scheme.params[1..];
        let mut args = vec![ty.clone()];
        args.extend(self.type_args(m, own, targs, name.span, "(..)")?);
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
        self.uses().fns.insert(super::scope::FnId(id));
        let f = &self.fns[id];
        if !f.public && !self.modules.within(self.module, f.module) {
            return Err(Error::ty(
                name.span,
                format!(
                    "method `{}` of `{}` is private to {}",
                    name.name,
                    d.self_ty,
                    self.modules.describe(f.module)
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
                    p,
                    name.span,
                    format!("`{p}` of the impl of `{}`", d.self_ty),
                    "annotate the type".into(),
                )
            })
            .collect();
        let own = &scheme.params[d.params.len()..];
        args.extend(self.type_args(&name.name, own, targs, name.span, "(..)")?);
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

impl Checker {
    /// The uses, for `generics::polymorphic_recursion`, of the methods a
    /// use in function `from`, with type parameters `from_params`, of `to`
    /// at `args`, at `span`, may run, if `to` is a trait's method: the
    /// method of each impl that could be for its `Self`, or the trait's
    /// default if the impl has none. An impl's type parameters are parts of
    /// what `Self` is, or of what its type parameters are.
    ///
    /// `Eq` has impls that aren't functions of the program, but do what one
    /// would. A built-in one of a reference or box compares what it points
    /// to, so `==` on a `&T` is `==` on a `T`. A derived one is `==` on
    /// each field: impl `k`'s is named `derived_eq[k]`, and its own uses are
    /// those (see `Checker::no_polymorphic_recursion`). Leaving either out
    /// would hide a polymorphic recursion through `==` from the check, and
    /// monomorphising it would never end (see `mono::Cx::need_eq`).
    pub(super) fn dispatch<'a>(
        &'a self,
        from: &'a str,
        from_params: &[String],
        to: &str,
        args: &[Type],
        derived_eq: &'a [Option<String>],
        span: Span,
    ) -> Vec<generics::Use<'a>> {
        let Some((tr, m)) = self.traits.traits.iter().find_map(|(t, d)| {
            let m = d.methods.iter().find(|m| m.symbol == to)?;
            Some((t, m))
        }) else {
            return Vec::new();
        };
        let (self_ty, own) = args.split_first().expect("`Self` first");
        let self_ty = match tr.as_str() {
            "Eq" => eq_subject(self_ty),
            _ => self_ty,
        };
        // A projection in it can be any type, so any impl may be for it,
        // and the impl's parameters are inside what it stands for, which
        // can be bigger than the type it's on by any amount: each is a
        // stand-in, a variable for `overlap`, whose parts are an unbounded
        // edge away from the parameters the projection mentions.
        let mut hidden: Vec<Hidden> = Vec::new();
        let self_ty = &hide_projections(self_ty, from_params, &mut hidden);
        let mut out = Vec::new();
        for (k, d) in self.traits.impls.iter().enumerate() {
            if d.trait_name.as_deref() != Some(tr) {
                continue;
            }
            let renamed: Vec<Type> = d
                .params
                .iter()
                .map(|p| Type::Param(format!("{p}'")))
                .collect();
            let header = Subst::new(&d.params, &renamed).apply(&d.self_ty);
            let mut vars: Vec<String> = d.params.iter().map(|p| format!("{p}'")).collect();
            vars.extend(from_params.iter().cloned());
            vars.extend(hidden.iter().map(|h| h.0.clone()));
            if !overlap(&header, self_ty, &vars) {
                continue;
            }
            let to: &str = match (d.methods.iter().find(|(n, _)| *n == m.name), &derived_eq[k]) {
                (Some((_, symbol)), _) => {
                    let f = self.fns.iter().find(|f| f.symbol == *symbol);
                    &f.expect("a method").symbol
                }
                (None, Some(eq)) => eq,
                // The default, at this `Self`. A derived impl has no
                // methods, nor do built-in ones.
                (None, None) => {
                    if !d.derived && m.provided {
                        let to = &self
                            .fns
                            .iter()
                            .find(|f| f.symbol == m.symbol)
                            .expect("a default")
                            .symbol;
                        out.push(generics::Use {
                            from,
                            to,
                            args: args.to_vec(),
                            inside: Vec::new(),
                            span,
                        });
                    }
                    continue;
                }
            };
            let mut found: Vec<Option<Type>> = vec![None; d.params.len()];
            let mut inside = Vec::new();
            split(
                &d.self_ty,
                self_ty,
                &d.params,
                (from_params, &hidden),
                &mut found,
                &mut inside,
            );
            let mut args: Vec<Type> = found
                .into_iter()
                .map(|t| shown_projections(&t.unwrap_or(Type::Unit), &hidden))
                .collect();
            args.extend(own.iter().cloned());
            out.push(generics::Use {
                from,
                to,
                args,
                inside,
                span,
            });
        }
        out
    }
}

/// Matches impl type `header`, with type parameters `params`, against
/// `t`, which mentions type parameters `outer`: each of `params` is either
/// a part of `t`, put in `found`, or inside what one of `outer` stands for,
/// some constructors below, which goes in `inside` as `(param, outer,
/// depth)`. Where they don't fit, nothing: the impl can't be for `t`.
///
/// `outer` comes with the stand-ins for the projections in `t` (see
/// `dispatch`): each of `params` inside one is inside what it stands for,
/// as much bigger than the parameters it mentions as an impl likes.
fn split(
    header: &Type,
    t: &Type,
    params: &[String],
    (outer, hidden): (&[String], &[Hidden]),
    found: &mut [Option<Type>],
    inside: &mut Vec<(usize, usize, usize)>,
) {
    match (header, t) {
        (Type::Param(p), _) if params.contains(p) => {
            let j = params.iter().position(|q| q == p).expect("it");
            found[j].get_or_insert_with(|| t.clone());
        }
        (h, Type::Param(p)) if hidden.iter().any(|x| x.0 == *p) => {
            let (_, _, mentions) = hidden.iter().find(|x| x.0 == *p).expect("it");
            let mut below = Vec::new();
            params_below(h, params, 0, &mut below);
            for (j, _) in below {
                for &i in mentions {
                    inside.push((j, i, usize::MAX));
                }
            }
        }
        (h, Type::Param(p)) if outer.contains(p) => {
            let i = outer.iter().position(|q| q == p).expect("it");
            let mut below = Vec::new();
            params_below(h, params, 0, &mut below);
            for (j, d) in below {
                if found[j].is_none() && !inside.iter().any(|x| x.0 == j) {
                    inside.push((j, i, d));
                }
            }
        }
        (h, t) if h.same_shape(t) => {
            for (a, b) in h.parts().zip(t.parts()) {
                split(a, b, params, (outer, hidden), found, inside);
            }
        }
        _ => {}
    }
}

/// Each of `params` in `t`, with how deep it is, `depth` and more down.
fn params_below(t: &Type, params: &[String], depth: usize, out: &mut Vec<(usize, usize)>) {
    match t {
        Type::Param(p) if params.contains(p) => {
            let j = params.iter().position(|q| q == p).expect("it");
            if !out.iter().any(|x| x.0 == j) {
                out.push((j, depth));
            }
        }
        t => t
            .parts()
            .for_each(|t| params_below(t, params, depth + 1, out)),
    }
}

/// The symbol of method `m` of trait `tr`: `<Show>::show`.
pub(super) fn trait_method_symbol(tr: &str, m: &str) -> String {
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
fn builtin_eq(t: &Type) -> bool {
    matches!(
        t,
        Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Ref(..) | Type::Box(_)
    )
}

/// The type arguments of struct or enum type `t`.
fn args_of(t: &Type) -> Vec<Type> {
    match t {
        Type::Adt(_, _, args) => args.clone(),
        _ => Vec::new(),
    }
}

/// Each field of struct or enum `d`, of kind `kind`, as "field `x`" or
/// "variant `V`", with its type.
pub(super) fn fields(kind: &AdtKind) -> Vec<(String, Type)> {
    match kind {
        AdtKind::Struct(fields) => fields
            .iter()
            .map(|(f, t)| (format!("field `{f}`"), t.clone()))
            .collect(),
        AdtKind::Enum(variants) => variants
            .iter()
            .flat_map(|(v, ts)| {
                ts.iter()
                    .map(move |t| (format!("variant `{v}`"), t.clone()))
            })
            .collect(),
    }
}

/// A trait impl must be for its type at *every* lifetime, as resolution
/// ignores lifetimes: it finds `impl Tr for Pair<'a, 'a>` for a
/// `Pair<'x, 'y>`, and the impl's methods, checked knowing the two are the
/// same, would then mix up references that live for different times. So
/// each lifetime in the impl's type is a parameter of its own, unbounded
/// and written once, and so is each type parameter, which could stand for a
/// type with lifetimes in it (`impl<T> Tr for Pair<T, T>` at `Pair<&'x
/// int, &'y int>`). That's Haskell 98's rule for instance heads, distinct
/// variables, with lifetimes counted. Accepting more would need resolution
/// to say what it needs of lifetimes, for the borrow checker to check.
///
/// Inherent impls don't need it: a method call there instantiates the
/// impl's own signature, lifetimes and all.
fn parametric(
    d: &ImplDecl,
    self_ty: &Type,
    params: &[String],
    bounds: &[Bound],
) -> Result<(), Error> {
    let err = |what: String| {
        Err(Error::ty(
            d.self_ty.span,
            format!(
                "{what}, but an impl of a trait must hold at every lifetime, as finding the impl \
                 for a type doesn't look at lifetimes"
            ),
        ))
    };
    let mut seen: Vec<&Region> = Vec::new();
    for r in self_ty.regions() {
        match r {
            Region::Static => return err(format!("`{self_ty}` says a lifetime is `'static`")),
            _ if seen.contains(&r) => {
                return err(format!("`{self_ty}` says two lifetimes are the same"));
            }
            _ => seen.push(r),
        }
    }
    for p in params {
        if occurrences(self_ty, p) > 1 {
            return err(format!(
                "`{self_ty}` says two of its parts are the same type `{p}`, which could have \
                 lifetimes in it that differ"
            ));
        }
    }
    if let Some(b) = bounds.first() {
        let b = match b {
            Bound::Region(x, y) => format!("{x}: {y}"),
            Bound::Param(p, r) => format!("{p}: {r}"),
            Bound::Proj(p, r) => format!("{p}: {r}"),
        };
        return err(format!("`{b}` says how long something lives"));
    }
    Ok(())
}

/// How many times `t` mentions type parameter `p`.
fn occurrences(t: &Type, p: &str) -> usize {
    match t {
        Type::Param(q) if q == p => 1,
        t => t.parts().map(|t| occurrences(t, p)).sum(),
    }
}

/// Does `t` mention type parameter `p`?
fn mentions(t: &Type, p: &str) -> bool {
    matches!(t, Type::Param(q) if q == p) || t.parts().any(|t| mentions(t, p))
}

/// Where `Self` is written in `t`, if it is.
fn mentions_self(t: &TypeExpr) -> Option<Span> {
    match &t.kind {
        TypeExprKind::Named(n, _) if n == "Self" => Some(t.span),
        TypeExprKind::Named(_, args) => args.iter().find_map(mentions_self),
        TypeExprKind::Fn(ps, r, e) => ps
            .iter()
            .find_map(mentions_self)
            .or_else(|| mentions_self(r))
            .or_else(|| e.iter().flat_map(EffectSet::types).find_map(mentions_self)),
        TypeExprKind::Ref(_, _, inner) | TypeExprKind::Proj(inner, ..) => mentions_self(inner),
        TypeExprKind::Impl(bounds) => bounds
            .iter()
            .flat_map(BoundItem::types)
            .find_map(mentions_self),
        TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => None,
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

/// Renames the lifetimes a method has of its own to `'0`, `'1`, .. in the
/// order they come up, for comparing two signatures up to their names.
/// Those in `fixed`, the impl's, keep theirs.
struct Canon<'a> {
    fixed: &'a [String],
    seen: Vec<Region>,
}

impl<'a> Canon<'a> {
    fn new(fixed: &'a [String]) -> Canon<'a> {
        Canon {
            fixed,
            seen: Vec::new(),
        }
    }

    fn region(&mut self, r: &Region) -> Region {
        match r {
            Region::Named(n) if self.fixed.contains(n) => r.clone(),
            Region::Named(n) if n.starts_with("'^") => r.clone(),
            Region::Named(_) | Region::Result(..) => {
                let i = match self.seen.iter().position(|s| s == r) {
                    Some(i) => i,
                    None => {
                        self.seen.push(r.clone());
                        self.seen.len() - 1
                    }
                };
                Region::Named(format!("'#{i}"))
            }
            r => r.clone(),
        }
    }

    fn ty(&mut self, t: &Type) -> Type {
        t.map_regions(&mut |r| self.region(r))
    }
}

/// Does an `Fn` bound in `g` say its calls may have effect parameter `p`?
fn in_fn_bound(g: &Generics, p: &str) -> bool {
    let bounds = g
        .bounds
        .iter()
        .map(|(_, b)| b)
        .chain(g.wheres.iter().map(|(_, b)| b));
    bounds
        .flatten()
        .flat_map(|b| &b.effects)
        .flat_map(|e| &e.items)
        .any(|e| matches!(&e.kind, EffectKind::Named(n) if n.name == p))
}

/// Rejects an impl's associated effect `t` with an effect parameter in it
/// that nothing decides: one that isn't in `self_ty`, nor in the effects
/// of an `Fn` bound, which give it its least value (see
/// `traits::impl_args`).
fn decided_effects(
    t: &Type,
    self_ty: &Type,
    eqs: &[(Type, Type)],
    span: Span,
) -> Result<(), Error> {
    for e in t.effect_items() {
        let Type::Param(p) = e else { continue };
        let in_bound = eqs.iter().any(|(lhs, rhs)| {
            crate::traits::is_sig(lhs)
                && matches!(rhs, Type::Fn(f) if f.effects.effect_items().contains(e))
        });
        if !in_bound && !mentions(self_ty, p) {
            return Err(Error::ty(
                span,
                format!(
                    "nothing decides what `{p}` is: say it's the effects of something an impl's                      type holds, with a bound like `F: Fn() with {p}`"
                ),
            ));
        }
    }
    Ok(())
}

/// Where a projection is written in `t`, if one is.
fn proj_in(t: &TypeExpr) -> Option<Span> {
    match &t.kind {
        TypeExprKind::Proj(..) => Some(t.span),
        TypeExprKind::Named(_, args) => args.iter().find_map(proj_in),
        TypeExprKind::Fn(ps, r, e) => {
            ps.iter()
                .find_map(proj_in)
                .or_else(|| proj_in(r))
                .or_else(|| {
                    let mut items = e.iter().flat_map(|e| &e.items);
                    let proj = items.find(|i| matches!(i.kind, EffectKind::Proj(..)));
                    proj.map(|i| i.span)
                })
        }
        TypeExprKind::Ref(_, _, inner) => proj_in(inner),
        TypeExprKind::Impl(bounds) => bounds.iter().flat_map(BoundItem::types).find_map(proj_in),
        TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => None,
    }
}

/// Checks that associated type `t`, written at `span` in an impl for
/// `self_ty` with type parameters `params`, only mentions parameters that
/// are in `self_ty` outside every function type (see `assoc_defs`).
fn constrained(t: &Type, self_ty: &Type, params: &[String], span: Span) -> Result<(), Error> {
    // The parameters and regions `self_ty` has outside function types.
    fn outside(t: &Type, types: &mut Vec<String>, regions: &mut Vec<Region>) {
        match t {
            Type::Fn(_) => {}
            Type::Param(p) => types.push(p.clone()),
            t => {
                match t {
                    Type::Ref(r, ..) => regions.push(r.clone()),
                    Type::Adt(_, rs, _) => regions.extend(rs.iter().cloned()),
                    _ => {}
                }
                t.parts().for_each(|p| outside(p, types, regions));
            }
        }
    }
    let (mut types, mut regions) = (Vec::new(), Vec::new());
    outside(self_ty, &mut types, &mut regions);
    let anywhere: Vec<&Region> = self_ty.regions();
    for r in t.regions() {
        match r {
            Region::Static => {}
            r if regions.contains(r) => {}
            r if anywhere.contains(&r) => {
                return Err(Error::ty(
                    span,
                    format!(
                        "`{r}` is only inside a function type in `{self_ty}`, so finding the \
                         impl doesn't say what it is: a function type's lifetimes can be any a \
                         call picks; an associated type can only use lifetimes that are in \
                         the type outside function types"
                    ),
                ));
            }
            r => {
                return Err(Error::ty(
                    span,
                    format!(
                        "`{r}` isn't in `{self_ty}`, so nothing would say what it is where \
                         this is used: `Out` would be a different type at each use; use a \
                         lifetime of the type, or `'static`"
                    ),
                ));
            }
        }
    }
    for p in params {
        if mentions(t, p) && !types.contains(p) {
            return Err(Error::ty(
                span,
                format!(
                    "`{p}` is only inside a function type in `{self_ty}`, and finding the impl \
                     doesn't look at what function types are generic over, so `{p}` may hold \
                     a lifetime that only means something inside it; an associated type can \
                     only use parameters that are in the type outside function types"
                ),
            ));
        }
    }
    Ok(())
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
fn terminates(t: &Type, self_ty: &Type, params: &[String], span: Span) -> Result<(), Error> {
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

/// Calls `f` with the type and trait of each projection in `t`.
fn each_proj<'t>(t: &'t Type, f: &mut impl FnMut(&'t Type, &'t str)) {
    if let Type::Proj(s, tr, _) = t {
        f(s, tr);
    }
    t.parts().for_each(|p| each_proj(p, f));
}

/// A projection in `t` on a type that isn't a parameter, or a projection
/// on one: one with no impl to work it out by.
fn rigid_on_type(t: &Type) -> Option<Type> {
    match t {
        Type::Proj(s, ..) if !matches!(**s, Type::Param(_) | Type::Proj(..)) => Some(t.clone()),
        t => t.parts().find_map(rigid_on_type),
    }
}

/// `t` with each projection in it a stand-in type parameter, `?p0`, `?p1`
/// and so on, added to `hidden` with the projection and the indices of the
/// parameters of `params` it mentions (see `dispatch`).
fn hide_projections(t: &Type, params: &[String], hidden: &mut Vec<Hidden>) -> Type {
    match t {
        Type::Proj(..) => {
            let name = format!("?p{}", hidden.len());
            let mentions = (0..params.len())
                .filter(|&i| mentions(t, &params[i]))
                .collect();
            hidden.push((name.clone(), t.clone(), mentions));
            Type::Param(name)
        }
        t => t.map(|p| hide_projections(p, params, hidden)),
    }
}

/// `t` with the stand-ins of `hide_projections` back as the projections
/// they stand for.
fn shown_projections(t: &Type, hidden: &[Hidden]) -> Type {
    match t {
        Type::Param(p) => match hidden.iter().find(|x| x.0 == *p) {
            Some((_, proj, _)) => proj.clone(),
            None => t.clone(),
        },
        t => t.map(|p| shown_projections(p, hidden)),
    }
}

/// The name of the associated type of the projection in `t` on `s` of
/// trait `tr`.
fn proj_name(t: &Type, s: &Type, tr: &str) -> Option<String> {
    match t {
        Type::Proj(x, y, name) if **x == *s && y == tr => Some(name.clone()),
        t => t.parts().find_map(|p| proj_name(p, s, tr)),
    }
}
