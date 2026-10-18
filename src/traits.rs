//! Traits and impls as the checker understands them, and *trait
//! resolution*: working out whether a type implements a trait, and if so
//! with which impl. The checker uses it to check bounds, and
//! monomorphisation to find the method each call runs (see
//! docs/06-traits.md).
//!
//! A trait is a *predicate on types*: `T: Show` says that `T` is one of the
//! types with a `show`. An impl is a rule for proving it, maybe from other
//! predicates: `impl<T: Show> Show for Option<T>` says `Option<T>: Show` if
//! `T: Show`. That's Horn clauses, with types for terms, and resolution is
//! the search Prolog does: find the impl whose type matches, then prove what
//! it needs. Two rules keep that search simple, both checked where impls
//! are collected (see `check/traits.rs`):
//!
//! - *Coherence*: at most one impl can match any type, so there's never a
//!   choice to make, or to get wrong. Which impl a call runs doesn't depend
//!   on what else is in scope.
//! - *Termination*: what an impl needs is about smaller types than what it
//!   proves, or about the same type for a trait that can't lead back to it,
//!   so the search always ends.

use crate::error::Span;
use crate::types::{Bound, Region, Scheme, Subst, Type};
use std::collections::HashMap;
use std::fmt;

/// `T: Show`: type `ty` implements trait `trait_name`. A bound says it's
/// so; an *obligation* is one the checker has to prove.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pred {
    pub ty: Type,
    pub trait_name: String,
}

impl fmt::Display for Pred {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.ty, self.trait_name)
    }
}

/// The traits built into the language: `Eq`, behind `==` and `!=`;
/// `Discard`, which says a type's values may be thrown away rather than
/// used up; and `Copy`, which says they're copied rather than moved, and
/// needs `Discard` (see docs/06a.1-linear-types.md).
pub const BUILTIN_TRAITS: &[&str] = &["Copy", "Discard", "Eq"];

/// A trait.
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Where its name is declared, unless it's built in.
    pub span: Option<Span>,
    /// Its supertraits, as written.
    pub supers: Vec<String>,
    pub methods: Vec<MethodDef>,
}

/// A method of a trait.
#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    /// What a call of it names, whichever type it's for: `<Show>::show`.
    /// Monomorphisation finds the impl's method from it.
    pub symbol: String,
    /// Generic over `Self` first, then the method's own type parameters,
    /// with `Self: Trait` among its bounds: `∀Self. Self: Show ⇒ fn(&Self)`.
    pub scheme: Scheme,
    /// Does it have a default body?
    pub provided: bool,
    /// Does it take `self`, `&self` or `&mut self`?
    pub receiver: bool,
    /// Where its name is declared, unless it's built in.
    pub span: Option<Span>,
}

/// An impl, of a trait or inherent.
#[derive(Debug, Clone)]
pub struct ImplDef {
    /// From the `impl` to the type it's for: where to point at it. For a
    /// derived impl, the `derive`.
    pub span: Span,
    pub trait_name: Option<String>,
    /// Its type and lifetime parameters, and the lifetime bounds on them.
    pub params: Vec<String>,
    pub lifetimes: Vec<String>,
    pub bounds: Vec<Bound>,
    /// The type it's for, which mentions `params`.
    pub self_ty: Type,
    /// What it needs, on `params`: `T: Show` in `impl<T: Show> ..`.
    pub preds: Vec<Pred>,
    /// The methods it defines, by name, with their symbols.
    pub methods: Vec<(String, String)>,
    /// Made by a `#[derive(..)]`, rather than written?
    pub derived: bool,
}

impl ImplDef {
    /// `Show for Option<T>`, or `Option<T>` for an inherent impl.
    pub fn header(&self) -> String {
        match &self.trait_name {
            Some(t) => format!("{t} for {}", self.self_ty),
            None => self.self_ty.to_string(),
        }
    }
}

/// Every trait and impl in a program, the built-in ones too.
#[derive(Debug, Clone, Default)]
pub struct Traits {
    pub traits: HashMap<String, TraitDef>,
    pub impls: Vec<ImplDef>,
}

/// How a type implements a trait.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// It's built into the language, like `int: Eq`.
    Builtin,
    /// By the impl at this index in `Traits::impls`, at these type
    /// arguments.
    Impl(usize, Vec<Type>),
    /// A bound says so: `T: Show` in `fn f<T: Show>`.
    Assumed,
}

impl Traits {
    /// The built-in traits, and nothing else yet.
    pub fn new() -> Traits {
        let mut traits = HashMap::new();
        traits.insert(
            "Discard".to_string(),
            TraitDef {
                span: None,
                supers: Vec::new(),
                methods: Vec::new(),
            },
        );
        // trait Copy: Discard {}
        traits.insert(
            "Copy".to_string(),
            TraitDef {
                span: None,
                supers: vec!["Discard".to_string()],
                methods: Vec::new(),
            },
        );
        // trait Eq { fn eq(&self, other: &Self) -> bool; }
        let this = || Type::Ref(Region::Named("'_0".into()), false, Box::new(param("Self")));
        let other = Type::Ref(Region::Named("'_1".into()), false, Box::new(param("Self")));
        let eq = MethodDef {
            name: "eq".into(),
            symbol: "<Eq>::eq".into(),
            scheme: Scheme {
                params: vec!["Self".into()],
                lifetimes: vec!["'_0".into(), "'_1".into()],
                bounds: Vec::new(),
                preds: vec![Pred {
                    ty: param("Self"),
                    trait_name: "Eq".into(),
                }],
                ty: Type::func(vec![this(), other], Type::Bool),
            },
            provided: false,
            receiver: true,
            span: None,
        };
        traits.insert(
            "Eq".to_string(),
            TraitDef {
                span: None,
                supers: Vec::new(),
                methods: vec![eq],
            },
        );
        Traits {
            traits,
            impls: Vec::new(),
        }
    }

    /// The traits that `T: name` implies, `name` first: it, its
    /// supertraits, theirs, and so on. A bound `T: Ord` gives `T` the
    /// methods of `Eq` too, if `Ord: Eq`.
    pub fn implied(&self, name: &str) -> Vec<String> {
        let mut out = vec![name.to_string()];
        let mut i = 0;
        while i < out.len() {
            if let Some(t) = self.traits.get(&out[i]) {
                for s in &t.supers {
                    if !out.contains(s) {
                        out.push(s.clone());
                    }
                }
            }
            i += 1;
        }
        out
    }

    /// The method `name` of trait `tr`, if it has one.
    pub fn method(&self, tr: &str, name: &str) -> Option<&MethodDef> {
        self.traits.get(tr)?.methods.iter().find(|m| m.name == name)
    }

    /// The impl of `tr` (or an inherent one, for `None`) whose type matches
    /// `ty`, with what its type parameters are for it to. Coherence makes
    /// that at most one.
    pub fn select(&self, ty: &Type, tr: Option<&str>) -> Option<(usize, Vec<Type>)> {
        self.impls.iter().enumerate().find_map(|(i, d)| {
            if d.trait_name.as_deref() != tr {
                return None;
            }
            let mut found = HashMap::new();
            if !matches(&d.self_ty, ty, &d.params, &mut found) {
                return None;
            }
            let args = d
                .params
                .iter()
                .map(|p| found.get(p).cloned().unwrap_or(Type::Unit))
                .collect();
            Some((i, args))
        })
    }

    /// Does `ty` implement `tr`, given that the bounds `env` hold? `ty`
    /// must have no type variables in it. If it doesn't, the obligation
    /// that fails, at the bottom of the search: for `Option<fn()>: Eq`,
    /// `fn(): Eq`.
    pub fn implements(&self, ty: &Type, tr: &str, env: &[Pred]) -> Result<Source, Pred> {
        self.implements_at(ty, tr, env, 0)
    }

    fn implements_at(
        &self,
        ty: &Type,
        tr: &str,
        env: &[Pred],
        depth: usize,
    ) -> Result<Source, Pred> {
        // Termination is checked when impls are collected, so this can only
        // go this deep on a bug.
        assert!(depth < 1000, "trait resolution of `{ty}: {tr}` doesn't end");
        let fail = || Pred {
            ty: ty.clone(),
            trait_name: tr.to_string(),
        };
        let all = |ts: &[&Type]| -> Result<Source, Pred> {
            for t in ts {
                self.implements_at(t, tr, env, depth + 1)?;
            }
            Ok(Source::Builtin)
        };
        // The built-in impls. Plain values may be thrown away, and so may
        // references, whatever they point to: dropping one doesn't drop
        // that. A box holds what it points to, so it may be if that may.
        match (tr, ty) {
            (
                "Discard",
                Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Fn(_) | Type::Ref(..),
            ) => return Ok(Source::Builtin),
            ("Discard", Type::Box(inner)) => return all(&[inner]),
            ("Copy", Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Fn(_)) => {
                return Ok(Source::Builtin);
            }
            ("Copy", Type::Ref(_, false, _)) => return Ok(Source::Builtin),
            ("Copy", Type::Ref(_, true, _) | Type::Box(_)) => return Err(fail()),
            ("Eq", Type::Int | Type::Bool | Type::Unit | Type::Never) => {
                return Ok(Source::Builtin);
            }
            // References and boxes compare what they point to.
            ("Eq", Type::Ref(_, _, inner) | Type::Box(inner)) => return all(&[inner]),
            ("Eq", Type::Fn(_)) => return Err(fail()),
            _ => {}
        }
        if let Type::Param(_) = ty {
            let assumed = env
                .iter()
                .any(|p| p.ty == *ty && self.implied(&p.trait_name).iter().any(|t| t == tr));
            return match assumed {
                true => Ok(Source::Assumed),
                false => Err(fail()),
            };
        }
        let Some((i, args)) = self.select(ty, Some(tr)) else {
            return Err(fail());
        };
        let d = &self.impls[i];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            self.implements_at(&s.apply(&p.ty), &p.trait_name, env, depth + 1)?;
        }
        Ok(Source::Impl(i, args))
    }
}

fn param(name: &str) -> Type {
    Type::Param(name.to_string())
}

/// Does `pat`, a type mentioning type parameters `params`, match `t` for
/// some choice of them? Adds what they must be to `found`. Regions don't
/// matter: an impl is for a type at every lifetime.
pub fn matches(pat: &Type, t: &Type, params: &[String], found: &mut HashMap<String, Type>) -> bool {
    match pat {
        Type::Param(p) if params.contains(p) => match found.get(p) {
            Some(prev) => prev.erased() == t.erased(),
            None => {
                found.insert(p.clone(), t.clone());
                true
            }
        },
        _ if !pat.same_shape(t) => false,
        _ => pat
            .parts()
            .zip(t.parts())
            .all(|(p, q)| matches(p, q, params, found)),
    }
}

/// Could `a` and `b`, which mention type parameters `params_a` and
/// `params_b` (different names, or renamed apart), be the same type, for
/// some choice of those? That's two impls *overlapping*. It's unification,
/// with the parameters for variables.
pub fn overlap(a: &Type, b: &Type, params: &[String]) -> bool {
    let mut s: HashMap<String, Type> = HashMap::new();
    unify(a, b, params, &mut s)
}

fn unify(a: &Type, b: &Type, params: &[String], s: &mut HashMap<String, Type>) -> bool {
    let walk = |t: &Type, s: &HashMap<String, Type>| {
        let mut t = t.clone();
        while let Type::Param(p) = &t {
            match s.get(p) {
                Some(u) => t = u.clone(),
                None => break,
            }
        }
        t
    };
    let (a, b) = (walk(a, s), walk(b, s));
    let var = |t: &Type| matches!(t, Type::Param(p) if params.contains(p));
    match (&a, &b) {
        (Type::Param(p), Type::Param(q)) if p == q => true,
        (Type::Param(p), t) | (t, Type::Param(p)) if var(&Type::Param(p.clone())) => {
            if occurs(p, t, s) {
                return false;
            }
            s.insert(p.clone(), t.clone());
            true
        }
        _ if a.same_shape(&b) => a
            .parts()
            .zip(b.parts())
            .all(|(p, q)| unify(p, q, params, s)),
        _ => false,
    }
}

fn occurs(p: &str, t: &Type, s: &HashMap<String, Type>) -> bool {
    match t {
        Type::Param(q) if q == p => true,
        Type::Param(q) => s.get(q).is_some_and(|u| occurs(p, u, s)),
        t => t.parts().any(|u| occurs(p, u, s)),
    }
}
