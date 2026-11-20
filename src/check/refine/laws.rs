//! Laws (docs/88-iterator-laws.md): what a trait `S` says of a method `m`
//! of another trait `P`, a supertrait of `S`'s, or one its `where Self: P`
//! names. Every type implementing `S` (and `P`) keeps it:
//!
//! - **Proved** where the impls are: for each impl of `S` and impl of `P`
//!   whose types can be the same, the `P` impl's `m` is walked again,
//!   under both impls' bounds, to keep the law's promises (`law_tables`
//!   pairs them, `Checker::keep_laws` walks). Its types must be the `S`
//!   impl's up to the names of their parameters, so that what's proved
//!   there is of every type both are of; and its body must be this
//!   package's, so that it's walked here.
//! - **Learned** at each call of `m`, of the trait's or an impl's, whose
//!   `Self` implements `S` where the call is (`Walk::law_posts`).
//!
//! That a call learns only what was proved of the body monomorphisation
//! runs is lemma L1 of docs/88-iterator-laws.md.

use super::{Checker, RefinedSig, Refinement, Translated, call_scoped};
use crate::ast::FnDecl;
use crate::check::source_name;
use crate::error::{Error, Span};
use crate::traits::{LawDef, Pred};
use crate::types::{Subst, Type};

/// What a law promises at a call of the method it's of, in the terms of
/// the function called: the trait's method, or an impl's.
#[derive(Debug, Clone)]
pub(in crate::check) struct LawPost {
    /// The law's trait: a call learns it if its `Self` implements it.
    pub(super) tr: String,
    /// The law's symbol.
    pub(super) law: String,
    pub(super) ret: Option<Refinement>,
    pub(super) ensures: Vec<Refinement>,
}

/// A law an impl's method keeps, for an impl of the law's trait whose
/// types are the method's impl's (`law_tables`).
#[derive(Debug, Clone)]
pub(in crate::check) struct LawCheck {
    /// The law's trait.
    tr: String,
    /// The law's symbol.
    law: String,
    /// The impl of the law's trait, for messages.
    span: Span,
    self_ty: Type,
    /// Its bounds, and equalities, in the method's impl's names.
    preds: Vec<Pred>,
    eqs: Vec<(Type, Type)>,
}

impl Checker {
    /// The laws of every trait, in a fixed order.
    fn all_laws(&self) -> Vec<(String, LawDef)> {
        let mut laws: Vec<(String, LawDef)> = (self.traits.traits.iter())
            .flat_map(|(tr, t)| t.laws.iter().map(move |l| (tr.clone(), l.clone())))
            .collect();
        laws.sort_by(|a, b| a.1.symbol.cmp(&b.1.symbol));
        laws
    }

    /// Works out, for each law, what calls of its method learn
    /// (`law_posts`), and which impls' methods must keep it, under which
    /// bounds (`law_checks`); and checks what a law may say.
    pub(in crate::check) fn law_tables(&mut self) -> Result<(), Error> {
        let laws = self.all_laws();
        for (tr, l) in &laws {
            let Some(sig) = self.refined.get(&l.symbol).cloned() else {
                continue;
            };
            // A law only promises: what a call through `P` proves is all
            // the method's body may assume, whatever else `Self` is.
            if let Some(j) = sig.params.iter().position(Option::is_some) {
                return Err(Error::ty(
                    l.span,
                    format!(
                        "a law can't refine parameter `{}`: it says only what `{}` promises, \
                         as its callers through `{}` prove only what it asks",
                        sig.names.get(j).map_or("it", String::as_str),
                        l.name,
                        source_name(&l.of)
                    ),
                ));
            }
            if sig.ret.is_none() && sig.ensures.is_empty() {
                continue;
            }
            // At a call through the trait `P`.
            let (Some(theirs), Some(m)) = (self.scheme_of(&l.symbol), self.scheme_of(&l.method))
            else {
                continue;
            };
            let (theirs, m) = (theirs.clone(), m.clone());
            let this = Type::Param("Self".into(), false);
            if let Some(t) = self.translated(&sig, &theirs, &m, &this, 1) {
                self.push_post(&l.method, tr, l, t);
            }
            // At a call of an impl's.
            let impls: Vec<(Type, usize, String)> = (self.traits.impls.iter())
                .filter(|d| d.trait_name.as_ref() == Some(&l.of))
                .filter_map(|d| {
                    let (_, s) = d.methods.iter().find(|(n, _)| *n == l.name)?;
                    Some((d.self_ty.clone(), d.params.as_slice().len(), s.clone()))
                })
                .collect();
            for (self_ty, n, symbol) in impls {
                let Some(mine) = self.fn_def(&symbol).map(|d| d.scheme.clone()) else {
                    continue;
                };
                if let Some(t) = self.translated(&sig, &theirs, &mine, &self_ty, n) {
                    self.push_post(&symbol, tr, l, t);
                }
            }
        }
        for (tr, l) in &laws {
            self.law_default(tr, l)?;
            self.law_pairs(tr, l)?;
        }
        Ok(())
    }

    /// For law `l` of trait `tr`, of a method with a default body: that
    /// body keeps it, for every `Self` implementing `tr`, as it's checked
    /// once, generically (an impl that writes its own is checked as any
    /// impl's, `law_pairs`). Only in the package of the method's trait,
    /// whose bodies are checked there, and so the law's must be too.
    fn law_default(&mut self, tr: &str, l: &LawDef) -> Result<(), Error> {
        let provided = self
            .traits
            .method(&l.of, &l.name)
            .is_some_and(|m| m.provided);
        if !provided {
            return Ok(());
        }
        // A law of an earlier package's was checked there.
        if l.package != self.package {
            return Ok(());
        }
        let here = (self.fn_index.get(&l.method)).is_some_and(|&i| i >= self.from.all_fns);
        if !here {
            return Err(Error::ty(
                l.span,
                format!(
                    "`{}` has a default body in `{}`, which is another package's: a law of it \
                     can only be in that package, where the body is checked",
                    l.name,
                    source_name(&l.of)
                ),
            ));
        }
        if let Some(sig) = self.refined.get(&l.symbol).cloned() {
            self.law_keepable(tr, l, &sig, &l.method)?;
        }
        let this = Type::Param("Self".into(), false);
        let check = LawCheck {
            tr: tr.to_string(),
            law: l.symbol.clone(),
            span: l.span,
            self_ty: this.clone(),
            preds: vec![Pred {
                ty: this,
                trait_name: tr.to_string(),
            }],
            eqs: Vec::new(),
        };
        self.law_checks
            .entry(l.method.clone())
            .or_default()
            .push(check);
        Ok(())
    }

    fn push_post(&mut self, at: &str, tr: &str, l: &LawDef, t: Translated) {
        let post = LawPost {
            tr: tr.to_string(),
            law: l.symbol.clone(),
            ret: t.ret,
            ensures: t.ensures,
        };
        self.law_posts.entry(at.to_string()).or_default().push(post);
    }

    /// Pairs each impl of law `l`'s trait `tr` with each impl of the trait
    /// its method is of whose types can be the same, one of them this
    /// package's: the method's body there keeps the law, under both
    /// impls' bounds (lemmas L1 and L3 of docs/88-iterator-laws.md).
    fn law_pairs(&mut self, tr: &str, l: &LawDef) -> Result<(), Error> {
        let impls = self.traits.impls.clone();
        let sig = self.refined.get(&l.symbol).cloned();
        for (si, s) in impls.iter().enumerate() {
            if s.trait_name.as_deref() != Some(tr) {
                continue;
            }
            for (pi, p) in impls.iter().enumerate() {
                if p.trait_name.as_ref() != Some(&l.of) {
                    continue;
                }
                if si < self.impls_from && pi < self.impls_from {
                    continue;
                }
                // `p`'s parameters, renamed apart from `s`'s.
                let renamed: Vec<Type> = p.params.tys_renamed(|q| format!("{q}'"));
                let p_ty = Subst::new(&p.params, &renamed).apply(&p.self_ty);
                let mut params = s.params.to_vec();
                params.extend(p.params.iter().map(|q| format!("{q}'")));
                if crate::traits::unifier(&s.self_ty, &p_ty, &params).is_none() {
                    continue;
                }
                // One that doesn't write its own has the default, which
                // keeps the law for every type (`law_default`). A derived
                // one has glue, which a law of its trait's method was
                // refused for (`Checker::law`).
                assert!(!p.derived, "a law of a method of a trait with glue");
                let Some((_, symbol)) = p.methods.iter().find(|(n, _)| *n == l.name) else {
                    let provided = (self.traits.method(&l.of, &l.name)).is_some_and(|m| m.provided);
                    assert!(
                        provided,
                        "an impl with neither its own method nor a default"
                    );
                    continue;
                };
                let (shown, of) = (source_name(tr), source_name(&l.of));
                let fail = |why: String| {
                    Err(Error::ty(
                        s.span,
                        format!(
                            "can't check that `{}` keeps `{shown}`'s law of `{}`: {why}",
                            s.self_ty, l.name
                        ),
                    )
                    .with_note(p.span, format!("its impl of `{of}`")))
                };
                if pi < self.impls_from {
                    return fail(format!(
                        "its impl of `{of}` is another package's, whose bodies are checked there"
                    ));
                }
                // The impl of `P` at `s`'s types, with what its parameters
                // are there: each one of `s`'s, a different one.
                let found = self.traits.select(&s.self_ty, Some(&l.of), &s.local_eqs());
                let Some((_, args)) = found.filter(|(i, _)| *i == pi) else {
                    return fail(format!(
                        "its impl of `{of}` is for more types than this one, or for others"
                    ));
                };
                let mut back: Vec<(String, Type)> = Vec::new();
                for (q, a) in p.params.iter().zip(&args) {
                    // An effect parameter's argument is the set of it.
                    let a = match a {
                        Type::Effects(items) if p.params.is_effect(q) && items.len() == 1 => {
                            &items[0]
                        }
                        a => a,
                    };
                    match a {
                        Type::Param(name, _) if !back.iter().any(|(n, _)| n == name) => {
                            back.push((name.clone(), p.params.arg(q)))
                        }
                        _ => {
                            return fail(format!(
                                "its impl of `{of}` is for more types than this one"
                            ));
                        }
                    }
                }
                if let Some(q) = s.params.iter().find(|q| !back.iter().any(|(n, _)| n == *q)) {
                    return fail(format!(
                        "its type parameter `{q}` isn't one of its impl of `{of}`'s"
                    ));
                }
                let (names, tys): (Vec<String>, Vec<Type>) = back.into_iter().unzip();
                let to_p = Subst::new(&names, &tys);
                let preds = (s.preds.iter())
                    .map(|q| Pred {
                        ty: to_p.apply(&q.ty),
                        trait_name: q.trait_name.clone(),
                    })
                    .collect();
                let eqs = (s.local_eqs().iter())
                    .map(|(a, b)| (to_p.apply(a), to_p.apply(b)))
                    .collect();

                if let Some(sig) = &sig {
                    self.law_keepable(tr, l, sig, symbol)?;
                }
                let check = LawCheck {
                    tr: tr.to_string(),
                    law: l.symbol.clone(),
                    span: s.span,
                    self_ty: s.self_ty.clone(),
                    preds,
                    eqs,
                };
                self.law_checks
                    .entry(symbol.clone())
                    .or_default()
                    .push(check);
            }
        }
        Ok(())
    }

    /// Checks impl method `symbol` can keep law `l`, of signature `sig`, as
    /// `inherit` checks it can keep its trait's: what the law promises of
    /// a parameter is of its value as passed, so it can't be `mut` (H2 of
    /// retro 15), and of what's behind a `&mut` at return, so it must be
    /// done with it then, as the method's trait's is.
    fn law_keepable(
        &self,
        tr: &str,
        l: &LawDef,
        sig: &RefinedSig,
        symbol: &str,
    ) -> Result<(), Error> {
        let Some(def) = self.fn_def(symbol) else {
            return Ok(());
        };
        let posts: Vec<&Expr> = (sig.ret.iter().chain(&sig.ensures))
            .flat_map(|r| r.preds())
            .collect();
        let named = |n: &str| posts.iter().any(|e| super::super::adts::names_var(e, n));
        let tr = source_name(tr);
        if let Some(j) = (sig.names.iter().zip(&def.muts)).position(|(n, m)| *m && named(n)) {
            let mine = &def.params[j];
            let fix = match mine.as_str() {
                "self" => "take a `mut` copy by another name, `let mut s = self;`".to_string(),
                _ => format!("write `let mut {mine} = {mine};` in the body"),
            };
            return Err(Error::ty(
                def.name.span,
                format!(
                    "`{mine}` can't be `mut` in `{}`: what `{tr}`'s law promises names it, as it \
                     was passed; {fix}",
                    def.name.name
                ),
            ));
        }
        let mine = match &def.scheme.ty {
            Type::Fn(f) => call_scoped(&f.params, &f.ret, &def.scheme.bounds),
            _ => Vec::new(),
        };
        let mine = match self.refined.get(symbol) {
            Some(own) => own.scoped.clone(),
            None => mine,
        };
        if let Some(j) = (sig.scoped.iter().zip(&mine)).position(|(t, m)| *t && !*m) {
            return Err(Error::ty(
                def.name.span,
                format!(
                    "`{}` can't keep what `{tr}`'s law promises of what's behind `{}` at return: \
                     what it gives back may still borrow it",
                    def.name.name,
                    sig.names.get(j).map_or("its parameter", String::as_str)
                ),
            ));
        }
        let _ = l;
        Ok(())
    }

    /// Walks the body of impl method `f` again for each law it keeps
    /// (`law_tables`): with the bounds of the law's trait's impl as well
    /// as its own, to keep the law's promises (lemma L1 of
    /// docs/88-iterator-laws.md). Only what fails is kept: what the walk
    /// proves with more bounds isn't so of every call.
    pub(super) fn keep_laws(&mut self, symbol: &str, f: &FnDecl) {
        let Some(checks) = self.law_checks.get(symbol).cloned() else {
            return;
        };
        for c in checks {
            let Some(post) = (self.law_posts.get(symbol))
                .and_then(|ps| ps.iter().find(|p| p.law == c.law))
                .cloned()
            else {
                continue;
            };
            let mut sig = match self.refined.get(symbol) {
                Some(sig) => sig.clone(),
                None => self.fresh_sig(symbol),
            };
            sig.ret = post.ret;
            sig.ensures = post.ensures;
            sig.asked = None;
            let env = self.adts.proj.env.clone();
            let local = self.adts.proj.local.clone();
            self.adts.proj.env.extend(c.preds.iter().cloned());
            self.adts.proj.local.extend(c.eqs.iter().cloned());
            let done = self.walk_mir_as(f, Some(sig));
            self.adts.proj.env = env;
            self.adts.proj.local = local;
            let name = c.law.rsplit("::").next().unwrap_or(&c.law).to_string();
            let tr = source_name(&c.tr);
            let note = format!(
                "`{tr}`'s law of `{name}`, which `{}` keeps, as it implements `{tr}`",
                c.self_ty
            );
            let mut errors = done.failed;
            if let Err(e) = done.result {
                errors.push(e);
            }
            for e in errors {
                self.proof_errors.push(e.with_note(c.span, note.clone()));
            }
        }
    }
}

use crate::ast::Expr;
