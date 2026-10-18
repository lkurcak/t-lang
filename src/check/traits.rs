//! Collecting traits and impls, and checking the rules that keep trait
//! resolution simple (see `traits.rs`): every impl makes sense, no two
//! overlap (*coherence*), and proving that a type implements a trait always
//! ends (*termination*). Also proving obligations, and finding the method a
//! call `x.m()` or a path `T::m` names.

use super::generics::{self, Lifetimes};
use super::{Checker, plural};
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::traits::{ImplDef, MethodDef, Pred, TraitDef, eq_subject, overlap};
use crate::types::{AdtKind, Bound, Region, Scheme, Subst, Type};
use std::collections::{HashMap, HashSet};

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
                methods: Vec::new(),
            };
            self.traits.traits.insert(name.name.clone(), def);
            self.info.symbols.insert(name.span, name.name.clone());
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

    /// The trait bounds in `g`, whose type parameters are `params`.
    pub(super) fn preds(&mut self, g: &Generics, params: &[String]) -> Result<Vec<Pred>, Error> {
        let mut out = Vec::new();
        for (subject, after) in &g.bounds {
            for b in after.iter().filter(|b| !b.name.starts_with('\'')) {
                let tr = self.trait_named(b)?;
                if params.contains(&subject.name) {
                    let pred = Pred {
                        ty: Type::Param(subject.name.clone()),
                        trait_name: tr,
                    };
                    if !out.contains(&pred) {
                        out.push(pred);
                    }
                }
            }
        }
        Ok(out)
    }

    /// Pass 1: each trait's supertraits and the signatures of its methods.
    /// No trait can be its own supertrait, even through others: then
    /// implementing it would need it implemented already.
    pub(super) fn collect_traits(&mut self, prog: &Program) -> Result<(), Error> {
        for t in &prog.traits {
            let mut supers = Vec::new();
            for s in &t.supers {
                let s = self.trait_named(s)?;
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            self.traits
                .traits
                .get_mut(&t.name.name)
                .expect("named")
                .supers = supers;
        }
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
                let mut scheme = self.method_scheme(f, &[], &[], &[], &[], None)?;
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

    /// The type of method `f`, in an impl with type parameters
    /// `impl_params`, lifetimes `impl_lifetimes`, bounds `impl_bounds` and
    /// `impl_preds`, for type `self_ty`, or in a trait if that's `None`,
    /// where `Self` is a type parameter. Generic over the impl's
    /// parameters and then its own.
    pub(super) fn method_scheme(
        &mut self,
        f: &FnDecl,
        impl_params: &[String],
        impl_lifetimes: &[String],
        impl_bounds: &[Bound],
        impl_preds: &[Pred],
        self_ty: Option<&Type>,
    ) -> Result<Scheme, Error> {
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
        preds.extend(self.preds(&f.generics, &own)?);
        // `Self` resolves as a type parameter, and in an impl is then
        // replaced by the impl's type.
        let mut with_self = params.clone();
        with_self.push("Self".into());
        let (param_tys, ret, lts) = self.signature(f, &with_self, &lifetimes)?;
        let mut ty = Type::func(param_tys, ret);
        if let Some(s) = self_ty {
            ty = Subst::new(&["Self".to_string()], std::slice::from_ref(s)).apply(&ty);
        }
        Ok(Scheme {
            params,
            lifetimes: lts,
            bounds,
            preds,
            ty,
        })
    }

    /// Pass 2: every impl's parameters, type and bounds, and that it's for
    /// something it can be.
    pub(super) fn collect_impls(&mut self, prog: &Program) -> Result<(), Error> {
        for d in &prog.impls {
            let params = generics::declare(&d.generics.types)?;
            let declared = generics::declare_lifetimes(&d.generics.lifetimes)?;
            let mut lts = match self.mono {
                true => Lifetimes::Any,
                false => Lifetimes::Params {
                    declared: &declared,
                    elided: Vec::new(),
                },
            };
            if let Some(s) = mentions_self(&d.self_ty) {
                return Err(Error::ty(
                    s,
                    "`Self` is the type an impl is for, so it can't be used to say what that is",
                ));
            }
            let self_ty = self.resolve_with(&d.self_ty, &params, &mut lts)?;
            let mut lifetimes = declared.clone();
            if let Lifetimes::Params { elided, .. } = lts {
                lifetimes.extend(elided);
            }
            let bounds = generics::bounds(&d.generics, &lifetimes, &params)?;
            let preds = self.preds(&d.generics, &params)?;
            let trait_name = match &d.trait_name {
                Some(t) => Some(self.trait_named(t)?),
                None => None,
            };
            let header = Span::new(d.span.start, d.self_ty.span.end);
            match (&trait_name, &self_ty) {
                (None, Type::Adt(..)) => {}
                (None, t) => {
                    return Err(Error::ty(
                        d.self_ty.span,
                        format!(
                            "an inherent impl can only be for a struct or enum, not `{t}`; to \
                             give `{t}` methods, declare a trait and implement that"
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
            // Each parameter must be decided by the type: otherwise nothing
            // could say what it is when the impl is used.
            for (p, decl) in params.iter().zip(&d.generics.types) {
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
                let symbol = self.fn_symbols[self.method_ids[&f.name.span].0].clone();
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
            self.impl_ids.push(self.traits.impls.len());
            self.traits.impls.push(ImplDef {
                span: header,
                trait_name,
                params,
                lifetimes,
                bounds,
                self_ty,
                preds,
                methods,
                derived: false,
            });
        }
        Ok(())
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
                    methods: Vec::new(),
                    derived: true,
                });
            }
        }
        // What monomorphisation made says what's `Copy` and `Eq` already.
        if self.mono {
            return Ok(());
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
                if preds != self.traits.impls[k].preds {
                    self.traits.impls[k].preds = preds;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for &(k, i, span, tr) in &derived {
            let def = &self.traits.impls[k];
            if tr != "Copy"
                || self
                    .traits
                    .implements(&def.self_ty, "Discard", &def.preds)
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
                    .implements(t, tr, &[])
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
        let (i, args) = self.traits.select(t, Some(tr)).ok_or_else(fail)?;
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
                let id = self.method_ids[&f.name.span];
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
                    .implements(&def.self_ty, &s, &def.preds)
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
            if tr == "Copy" || tr == "Discard" {
                let Type::Adt(symbol, ..) = &def.self_ty else {
                    unreachable!("checked")
                };
                let adt = &self.adts[symbol];
                let s = Subst::new(&adt.params, &args_of(&def.self_ty));
                for (what, t) in fields(&adt.kind) {
                    let t = s.apply(&t);
                    if self.traits.implements(&t, tr, &def.preds).is_err() {
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
        self.coherence()?;
        self.termination()
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
        args.extend(own.iter().cloned().map(Type::Param));
        let s = Subst::new(&want.scheme.params, &args);
        let wanted = s.apply(&want.scheme.ty);
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
                ty: s.apply(&p.ty),
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

    /// Does `t`, which has no type variables in it, implement `tr`, with
    /// the bounds in scope? Monomorphisation's output says which of its
    /// types do (see `derive`).
    pub(crate) fn is_copy(&self, t: &Type) -> bool {
        match self.mono {
            true => crate::types::is_copy(&self.adts, t, false),
            false => self.traits.implements(t, "Copy", &self.env).is_ok(),
        }
    }

    /// May a value of type `t`, which has no type variables in it, be
    /// thrown away, with the bounds in scope? Only asked before
    /// monomorphisation: the borrow checker asks it, and it's done by then.
    pub(crate) fn is_discard(&self, t: &Type) -> bool {
        self.traits.implements(t, "Discard", &self.env).is_ok()
    }

    /// What `t`, which isn't `Discard`, would need to be, for messages to
    /// say: a type parameter, which a bound can make so, or a struct or
    /// enum that deriving `Discard` would make so, if its fields allow
    /// that. `None` if it's neither, as a fix that wouldn't work is no fix.
    pub(crate) fn missing_discard(&self, t: &Type) -> Option<Type> {
        let bad = self.traits.implements(t, "Discard", &self.env).err()?;
        let Type::Adt(symbol, ..) = &bad.ty else {
            return matches!(bad.ty, Type::Param(_)).then_some(bad.ty);
        };
        if self.traits.select(&bad.ty, Some("Discard")).is_some() {
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
            methods: Vec::new(),
            derived: true,
        });
        let fields_may = fields(&adt.kind)
            .iter()
            .all(|(_, f)| traits.implements(f, "Discard", &assumed).is_ok());
        (fields_may && traits.implements(t, "Discard", &self.env).is_ok()).then_some(bad.ty)
    }

    /// Requires `pred`, at `span` because of `why`: now, if its type is
    /// known, or else once the function's types are.
    pub(super) fn oblige(&mut self, pred: Pred, span: Span, why: Why) -> Result<(), Error> {
        let ty = self.solved(&pred.ty);
        let o = Obligation {
            pred: Pred {
                ty,
                trait_name: pred.trait_name,
            },
            span,
            why,
        };
        match super::unify::has_vars(&o.pred.ty) {
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
        let bad = match self.traits.implements(ty, tr, &self.env) {
            Ok(_) => return Ok(()),
            Err(bad) => bad,
        };
        // How to fix what doesn't hold, if it's simple, or why it can't be.
        let at_bottom = bad == o.pred;
        let fix = |p: &Pred| match (&p.ty, p.trait_name.as_str()) {
            (Type::Param(t), tr) => format!("; add a bound: `{t}: {tr}`"),
            (Type::Fn(_), "Eq") if at_bottom => ": functions can't be compared".to_string(),
            (Type::Fn(_), "Eq") => ", and functions can't be compared".to_string(),
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
        };
        Err(Error::ty(o.span, message))
    }

    /// At the end of a function, the obligations that waited for its types.
    pub(super) fn prove_all(&mut self) -> Result<(), Error> {
        for mut o in std::mem::take(&mut self.obligations) {
            o.pred.ty = self.solved(&o.pred.ty);
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
        super::each_adt(t, &mut |name, args| {
            if let Some(def) = self.adts.get(name)
                && !def.preds.is_empty()
            {
                let s = Subst::new(&def.params, args);
                for p in &def.preds {
                    let Type::Param(param) = &p.ty else {
                        unreachable!("bounds are on type parameters")
                    };
                    let why = Why::Bound(super::source_name(name), param.clone());
                    let pred = Pred {
                        ty: s.apply(&p.ty),
                        trait_name: p.trait_name.clone(),
                    };
                    preds.push((pred, why));
                }
            }
        });
        for (pred, why) in preds {
            self.oblige(pred, span, why)?;
        }
        Ok(())
    }

    /// `well_formed`, for a type with no variables in it, outside a body.
    pub(super) fn well_formed_now(&mut self, t: &Type, span: Span) -> Result<(), Error> {
        if self.mono {
            return Ok(());
        }
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
            (None, Type::Param(_)) => {
                let mut out = Vec::new();
                for p in self.env.iter().filter(|p| p.ty == ty) {
                    for t in self.traits.implied(&p.trait_name) {
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
                out.sort();
                out.dedup();
                out
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
        let def = self.traits.method(&tr, m).expect("it has it").clone();
        let own = &def.scheme.params[1..];
        let mut args = vec![ty.clone()];
        args.extend(self.type_args(m, own, targs, name.span, "(..)")?);
        let fn_ty = Subst::new(&def.scheme.params, &args).apply(&def.scheme.ty);
        // Its own bounds, besides `Self: Trait`.
        for p in &def.scheme.preds[1..] {
            let s = Subst::new(&def.scheme.params, &args);
            let Type::Param(param) = &p.ty else {
                unreachable!("bounds are on type parameters")
            };
            let why = Why::Bound(m.clone(), param.clone());
            let pred = Pred {
                ty: s.apply(&p.ty),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, name.span, why)?;
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
        let (scheme, def) = (f.scheme.clone(), f.name.span);
        let receiver = self.receivers[id];
        // The impl's parameters are worked out from the type; the method's
        // own are written or inferred.
        let mut args: Vec<Type> = d
            .params
            .iter()
            .map(|p| {
                self.fresh(
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
        for p in &scheme.preds {
            let Type::Param(param) = &p.ty else {
                unreachable!("bounds are on type parameters")
            };
            let why = Why::Bound(format!("{}::{}", d.self_ty, name.name), param.clone());
            let pred = Pred {
                ty: s.apply(&p.ty),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, name.span, why)?;
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
                from_params,
                &mut found,
                &mut inside,
            );
            let mut args: Vec<Type> = found.into_iter().map(|t| t.unwrap_or(Type::Unit)).collect();
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
fn split(
    header: &Type,
    t: &Type,
    params: &[String],
    outer: &[String],
    found: &mut [Option<Type>],
    inside: &mut Vec<(usize, usize, usize)>,
) {
    match (header, t) {
        (Type::Param(p), _) if params.contains(p) => {
            let j = params.iter().position(|q| q == p).expect("it");
            found[j].get_or_insert_with(|| t.clone());
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
                split(a, b, params, outer, found, inside);
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
        TypeExprKind::Fn(ps, r) => ps
            .iter()
            .find_map(mentions_self)
            .or_else(|| mentions_self(r)),
        TypeExprKind::Ref(_, _, inner) => mentions_self(inner),
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
            Region::Named(_) => {
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
