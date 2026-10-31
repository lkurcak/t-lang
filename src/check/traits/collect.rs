//! Collecting traits and impls: their names, signatures and associated
//! types, and the rules each impl header must keep.

use super::termination::terminates;
use super::{mentions, occurrences, trait_method_symbol};
use crate::ast::*;
use crate::check::generics::{self, Lifetimes};
use crate::check::{Checker, plural};
use crate::error::{Edit, Error, Fix, Span};
use crate::traits::{AssocDecl, ImplDef, MethodDef, Pred, TraitDef};
use crate::traits::{COPY, DISCARD, FN_ONCE, SIGNED, UNSIGNED};
use crate::types::{AdtKind, Bound, Region, Scheme, Subst, Type};

impl Checker {
    /// Pass 0: the names of the traits, so that anything can mention any
    /// of them.
    pub(in crate::check) fn trait_names(&mut self, prog: &Program) -> Result<(), Error> {
        for t in &prog.traits[self.from.traits..] {
            let name = &t.name;
            if self.traits.traits.contains_key(&name.name) {
                let message = format!("trait `{}` is already defined", name.name);
                return Err(Error::ty(name.span, message));
            }
            let intrinsic = crate::traits::BUILTIN_TRAITS.contains(&name.name.as_str());
            match t.intrinsic {
                Some(at) if !self.modules.in_library(t.module) => {
                    return Err(crate::check::not_library(at));
                }
                Some(_) if !intrinsic => {
                    let message = format!("there's no intrinsic trait `{}`", name.name);
                    return Err(Error::ty(name.span, message));
                }
                None if intrinsic => {
                    let message = format!("`{}` must be `#[intrinsic]`", name.name);
                    return Err(Error::ty(name.span, message));
                }
                _ => {}
            }
            // The checker gives the integer types these, and nothing else
            // has them: so they're sealed, and must say so.
            let sealed = crate::traits::SEALED_TRAITS.contains(&name.name.as_str());
            if sealed && t.sealed.is_none() {
                let message = format!("`{}` must be `sealed`", name.name);
                return Err(Error::ty(name.span, message));
            }
            if t.sealed.is_some() {
                self.traits.sealed.insert(name.name.clone());
            }
            // `FnOnce`'s signature: what a bound like `Fn(i64) -> i64`
            // says it is. Its name can't be written, so nothing else can.
            let assoc = match name.name == FN_ONCE {
                true => vec![AssocDecl {
                    name: crate::traits::SIG.into(),
                    effect: false,
                    bounds: Vec::new(),
                    span: name.span,
                }],
                false => Vec::new(),
            };
            let def = TraitDef {
                span: Some(name.span),
                supers: Vec::new(),
                assoc,
                methods: Vec::new(),
            };
            self.traits.traits.insert(name.name.clone(), def);
            self.info.symbols.insert(name.id, name.name.clone());
        }
        // Each trait's supertraits and associated types, which a type
        // anywhere can mention, as `T::Item`.
        for t in &prog.traits[self.from.traits..] {
            let mut supers = Vec::new();
            for s in &t.supers {
                let s = self.trait_named(s)?;
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            let mut assoc = std::mem::take(
                &mut self
                    .traits
                    .traits
                    .get_mut(&t.name.name)
                    .expect("named")
                    .assoc,
            );
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
                    effect: false,
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
                    effect: true,
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
    pub(in crate::check) fn trait_named(&mut self, name: &Ident) -> Result<String, Error> {
        match self.traits.traits.get(&name.name) {
            Some(t) => {
                let kind = crate::check::NoteKind::Trait(name.name.clone());
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

    /// Pass 1: each trait's supertraits and the signatures of its methods.
    /// No trait can be its own supertrait, even through others: then
    /// implementing it would need it implemented already.
    pub(in crate::check) fn collect_traits(&mut self, prog: &Program) -> Result<(), Error> {
        for t in &prog.traits[self.from.traits..] {
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
        for t in &prog.traits[self.from.traits..] {
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
                scheme.params.insert_type("Self".into());
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
                    effects_at: Some(crate::check::effects::written_at(f)),
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
    pub(in crate::check) fn method_scheme(
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
        let own = generics::declare(&f.generics)?;
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
        let params = impl_params.concat(&own);
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
        with_self.push_type("Self".into());
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
        self.sig_self = self_ty.cloned();
        let signature = self.signature(f, &with_self, &lifetimes, opaques);
        self.sig_self = None;
        let (param_tys, ret, effects, lts) = signature?;
        let mut ty = Type::func_with(param_tys, ret, effects);
        if let Some(s) = self_ty {
            let s = Subst::new(&["Self".to_string()], std::slice::from_ref(s));
            ty = self.adts.normalise(&s.apply(&ty));
            // `Self` is `[T]` in a slice's own methods, which take it by
            // reference.
            if let Some(u) = ty.unsized_part() {
                return Err(generics::unsized_slice(f.name.span, u));
            }
            // What its opaque types' bounds say of `Self` is of the impl's
            // type too.
            for o in crate::check::opaques_of(&ty) {
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
    pub(in crate::check) fn collect_impls(&mut self, prog: &Program) -> Result<(), Error> {
        self.collecting = true;
        let result = self.collect_impls_in(prog);
        self.collecting = false;
        result
    }

    fn collect_impls_in(&mut self, prog: &Program) -> Result<(), Error> {
        for d in &prog.impls[self.from.impls..] {
            let params = generics::declare(&d.generics)?;
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
            let self_ty = match &d.self_ty.kind {
                // `impl<T> [T]`: a slice's own methods, which only the
                // built-in libraries give it, each taking it by reference.
                TypeExprKind::Slice(elem) if d.trait_name.is_none() => {
                    Type::Slice(Box::new(self.resolve_with(elem, &params, &mut lts)?))
                }
                // `impl str`, and in the built-in libraries, `impl Ord for
                // str`: they take care that each method of the trait has
                // `Self` behind a reference, or a bound no `str` meets.
                TypeExprKind::Named(n, args)
                    if n == "str"
                        && args.is_empty()
                        && (d.trait_name.is_none() || self.modules.in_library(d.module)) =>
                {
                    Type::Str
                }
                _ => self.resolve_with(&d.self_ty, &params, &mut lts)?,
            };
            let mut lifetimes = declared.clone();
            if let Lifetimes::Params { elided, .. } = lts {
                lifetimes.extend(elided);
            }
            let bounds = generics::bounds(&d.generics, &lifetimes, &params)?;
            // What an impl is for is in its type, so a `where` clause can't
            // bound that.
            if let Some(s) = d.generics.wheres.iter().find_map(|(t, _)| mentions_self(t)) {
                return Err(Error::ty(
                    s,
                    "an impl's `where` clause can only bound a type parameter, or an associated \
                     type of one, not `Self`",
                ));
            }
            let (preds, eqs) = self.preds(&d.generics, &params, &lifetimes, true)?;
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
                (None, Type::Slice(_) | Type::Array(..) | Type::Str | Type::Char)
                    if self.modules.in_library(d.module) => {}
                (None, t) => {
                    return Err(Error::ty(
                        d.self_ty.span,
                        format!(
                            "an inherent impl can only be for a struct or enum, not `{t}`; to \
                             give `{t}` methods, declare a trait and implement that"
                        ),
                    ));
                }
                (Some(tr), _) if crate::traits::SEALED_TRAITS.contains(&tr.as_str()) => {
                    let which = match tr.as_str() {
                        SIGNED => "the signed integer types",
                        UNSIGNED => "the unsigned integer types",
                        _ => "the integer types",
                    };
                    return Err(Error::ty(
                        header,
                        format!(
                            "`{tr}` is built in: only {which} implement it, so generic code over \
                             it may count on what they have in common"
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
                // A `discard` of its own: its module's to write, as only its
                // module may take its values apart, skipping it (lemma D4
                // of docs/20-destructors.md). With none, it says no more
                // than deriving it.
                (Some(tr), Type::Adt(symbol, ..)) if tr == DISCARD && !d.methods.is_empty() => {
                    let def = self.types.iter().find(|t| t.symbol == *symbol);
                    if let Some(def) = def
                        && def.module != d.module
                    {
                        return Err(Error::ty(
                            header,
                            format!(
                                "`{self_ty}`'s `discard` must be written in {}, where it's \
                                 declared: only there may its values be taken apart",
                                self.modules.describe(def.module)
                            ),
                        ));
                    }
                }
                (Some(tr), t @ Type::Adt(..)) if tr == DISCARD => {
                    let e = Error::ty(
                        header,
                        format!(
                            "an impl of `{tr}` gives a `discard` of its own; for one that drops \
                             each field, derive it, with `#[derive({tr})]`"
                        ),
                    );
                    return Err(match self.derive_fix(prog, d, tr, t) {
                        Some(fix) => e.with_fix(fix),
                        None => e,
                    });
                }
                // Only derived, so whether a type is one follows from its
                // fields alone: a proof of it needs only more of the same,
                // never another trait (see `derive`).
                (Some(tr), t) if tr == COPY => {
                    let e = Error::ty(
                        header,
                        format!(
                            "`{tr}` can only be derived, with `#[derive({tr})]` on a struct or \
                             enum, so that whether a type is `{tr}` follows from its fields alone"
                        ),
                    );
                    return Err(match self.derive_fix(prog, d, tr, t) {
                        Some(fix) => e.with_fix(fix),
                        None => e,
                    });
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
            // could say what it is when the impl is used. Or by an equality
            // bound on a parameter that is, as what that's a function of
            // the type, or, for an effect parameter, by an `Fn` bound, as
            // the least effects that make it hold (see `traits::impl_args`).
            for (p, decl) in params.iter().zip(&d.generics.types) {
                if mentions(&self_ty, p) {
                    continue;
                }
                if params.is_effect(p) {
                    if in_fn_bound(&d.generics, p) {
                        continue;
                    }
                    return Err(Error::ty(
                        decl.span,
                        format!(
                            "effect parameter `{p}` isn't decided by anything: say it's the \
                             effects of something the impl's type holds, with a bound like \
                             `F: Fn() with {p}`"
                        ),
                    ));
                }
                match crate::traits::decider(&self_ty, &params, &eqs, p) {
                    Some(k) => terminates(&eqs[k].0, &self_ty, &params, decl.span)?,
                    None => {
                        return Err(Error::ty(
                            decl.span,
                            format!(
                                "type parameter `{p}` isn't in `{self_ty}`, nor decided by a \
                                 bound, so nothing could say what it is where the impl is used; \
                                 remove it, or say what it is with a bound on a parameter that \
                                 is, like `I: Iterator<Item = {p}>`"
                            ),
                        ));
                    }
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

    /// The fix for impl `d` of `tr`, which can only be derived, for type
    /// `t`: derive it instead, if `t` is a struct or enum of this package.
    fn derive_fix(&self, prog: &Program, d: &ImplDecl, tr: &str, t: &Type) -> Option<Fix> {
        let Type::Adt(symbol, ..) = t else {
            return None;
        };
        let i = self.types.iter().position(|t| t.symbol == *symbol)?;
        let (decl, _) = *prog.all_types().get(i)?;
        if i < self.from.all_types {
            return None;
        }
        let add = match decl.derives().first() {
            Some(derive) => {
                let last = derive.names.last()?;
                Edit::insert(last.span.end, format!(", {tr}"))
            }
            None => {
                let span = match decl {
                    TypeDecl::Struct(s) => s.vis.span().unwrap_or(s.span),
                    TypeDecl::Enum(e) => e.vis.span().unwrap_or(e.span),
                    TypeDecl::Alias(_) => return None,
                };
                Edit::insert(span.start, format!("#[derive({tr})]\n"))
            }
        };
        let title = format!("derive `{tr}` instead");
        Some(Fix::new(title, vec![add, Edit::delete(d.span)]))
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
    ///   over: `impl<A, B> Tr for fn(A) -> B` is found for `fn(&i64) ->
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
        params: &crate::types::Params,
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
            if !decls.iter().any(|x| x.name == *n && !x.effect) {
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
            let mut with_self = params.clone();
            with_self.push_type("Self".into());
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
            // A parameter a bound decides is a type that means something
            // anywhere (see `traits::impl_args`).
            let header: Vec<String> = params
                .iter()
                .filter(|p| mentions(self_ty, p))
                .cloned()
                .collect();
            constrained(&t, self_ty, &header, a.ty.span)?;
            terminates(&t, self_ty, params, a.ty.span)?;
            out.push((n.clone(), t));
        }
        for e in &d.effects {
            let n = &e.name.name;
            if !decls.iter().any(|x| x.name == *n && x.effect) {
                return Err(Error::ty(
                    e.name.span,
                    format!(
                        "`{}` has no associated effect `{n}`",
                        crate::check::source_name(tr)
                    ),
                ));
            }
            if out.iter().any(|(m, _)| m == n) {
                return Err(Error::ty(
                    e.name.span,
                    format!("associated effect `{n}` is defined twice"),
                ));
            }
            let mut with_self = params.clone();
            with_self.push_type("Self".into());
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
            missing.into_iter().partition(|x| x.effect);
        for (missing, what) in [(types, "associated type"), (effects, "associated effect")] {
            if missing.is_empty() {
                continue;
            }
            let names: Vec<String> = missing.iter().map(|x| format!("`{}`", x.name)).collect();
            return Err(Error::ty(
                header,
                format!(
                    "this impl of `{}` for `{self_ty}` is missing {}: {}",
                    crate::check::source_name(tr),
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

    /// Once every impl of the package is collected, what each projection
    /// stands for is known: the types already resolved, of its fields and
    /// of its traits' methods, get theirs worked out. Those of the packages
    /// before it are as they were: what a projection in them stands for
    /// can't change, as a later package's impls are of its own types or
    /// traits (see `orphan`).
    pub(in crate::check) fn projections_known(&mut self, prog: &Program) {
        let local = std::mem::take(&mut self.adts.proj.local);
        self.adts.proj = crate::traits::Projections::new(&self.traits);
        let proj = self.adts.proj.clone();
        let own: std::collections::HashSet<&str> = (self.types[self.from.all_types..].iter())
            .map(|t| t.symbol.as_str())
            .collect();
        let defs = self
            .adts
            .iter_mut()
            .filter(|(s, _)| own.contains(s.as_str()));
        for (_, def) in defs {
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
        for t in &prog.traits[self.from.traits..] {
            let def = self.traits.traits.get_mut(&t.name.name).expect("collected");
            for m in &mut def.methods {
                m.scheme.ty = proj.normalise(&m.scheme.ty);
            }
        }
        self.adts.proj.local = local;
    }
}

/// What a method of an impl is inside of: the impl's parameters, bounds
/// and equality bounds, which it's generic over too. Nothing, in a trait.
#[derive(Debug, Clone, Default)]
pub(in crate::check) struct Outer {
    pub(in crate::check) params: crate::types::Params,
    pub(in crate::check) lifetimes: Vec<String>,
    pub(in crate::check) bounds: Vec<Bound>,
    pub(in crate::check) preds: Vec<Pred>,
    pub(in crate::check) eqs: Vec<(Type, Type)>,
}

/// A trait impl must be for its type at *every* lifetime, as resolution
/// ignores lifetimes: it finds `impl Tr for Pair<'a, 'a>` for a
/// `Pair<'x, 'y>`, and the impl's methods, checked knowing the two are the
/// same, would then mix up references that live for different times. So
/// each lifetime in the impl's type is a parameter of its own, unbounded
/// and written once, and so is each type parameter, which could stand for a
/// type with lifetimes in it (`impl<T> Tr for Pair<T, T>` at `Pair<&'x
/// i64, &'y i64>`). That's Haskell 98's rule for instance heads, distinct
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
        TypeExprKind::Ref(_, _, inner)
        | TypeExprKind::Slice(inner)
        | TypeExprKind::Proj(inner, ..)
        | TypeExprKind::Refined(_, inner, _) => mentions_self(inner),
        TypeExprKind::Array(inner, n) => mentions_self(inner).or_else(|| mentions_self(n)),
        TypeExprKind::Impl(bounds) => bounds
            .iter()
            .flat_map(BoundItem::types)
            .find_map(mentions_self),
        TypeExprKind::Unit
        | TypeExprKind::Never
        | TypeExprKind::Lifetime(_)
        | TypeExprKind::Nat(_) => None,
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
        TypeExprKind::Ref(_, _, inner)
        | TypeExprKind::Slice(inner)
        | TypeExprKind::Array(inner, _)
        | TypeExprKind::Refined(_, inner, _) => proj_in(inner),
        TypeExprKind::Impl(bounds) => bounds.iter().flat_map(BoundItem::types).find_map(proj_in),
        TypeExprKind::Unit
        | TypeExprKind::Never
        | TypeExprKind::Lifetime(_)
        | TypeExprKind::Nat(_) => None,
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
