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
use std::sync::Arc;

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
/// used up; `Copy`, which says they're copied rather than moved, and
/// needs `Discard` (see docs/06a.1-linear-types.md); and `Int`, which only
/// the integer types implement, and says the arithmetic operators work on
/// a type (see docs/11-integers.md).
pub const BUILTIN_TRAITS: &[&str] = &[
    "Copy", "Discard", "Eq", "Fn", "FnMut", "FnOnce", "Int", "Signed", "Unsigned",
];

/// The built-in traits no impl can add to: the integer types are all
/// there are, and generic code may count on what they have in common
/// (see docs/11-integers.md).
pub const SEALED_TRAITS: &[&str] = &["Int", "Signed", "Unsigned"];

/// The traits of what can be called: `F: Fn(i64) -> i64` says a value of
/// type `F` can be called with an `i64`, giving an `i64`. Functions and
/// closures implement them, and nothing else can (see
/// docs/06c-closures.md). Each is stronger than the next: `trait Fn:
/// FnMut` and `trait FnMut: FnOnce`.
pub const FN_TRAITS: &[&str] = &["Fn", "FnMut", "FnOnce"];

/// The associated type of `FnOnce` that a bound like `Fn(i64) -> i64`
/// says is `fn(i64) -> i64`: what a call takes and gives. Its name can't
/// be written, so only such bounds can say what it is.
pub const SIG: &str = "(signature)";

/// How a call of a callable value gets at it: `Fn` borrows it, `FnMut`
/// borrows it mutably, and `FnOnce` takes it. In that order, each allows
/// fewer calls than the one before, and needs less of the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FnKind {
    Fn,
    FnMut,
    FnOnce,
}

impl FnKind {
    pub fn name(self) -> &'static str {
        match self {
            FnKind::Fn => "Fn",
            FnKind::FnMut => "FnMut",
            FnKind::FnOnce => "FnOnce",
        }
    }

    pub fn of(trait_name: &str) -> Option<FnKind> {
        match trait_name {
            "Fn" => Some(FnKind::Fn),
            "FnMut" => Some(FnKind::FnMut),
            "FnOnce" => Some(FnKind::FnOnce),
            _ => None,
        }
    }
}

/// What a closure that captures is, as a type: a struct of what it
/// captures, with an impl of the `Fn` traits whose body is the closure's.
/// Its type is `Type::Adt` of its symbol, with every type parameter in
/// scope where it's written for type arguments, and a lifetime for each
/// region in its captures' types and its signature, which are its
/// lifetime parameters. It's the same type each time the closure
/// expression runs, as a struct is, but for those regions.
#[derive(Debug, Clone)]
pub struct ClosureDef {
    /// The type parameters in scope where it's written.
    pub params: Vec<String>,
    /// Its lifetime parameters: none until its captures are worked out,
    /// at the end of the function it's in.
    pub lifetimes: Vec<String>,
    /// What it takes and gives, mentioning `params` and `lifetimes`.
    pub sig: crate::types::FnType,
    /// Which of the `Fn` traits it implements, the strongest: worked out
    /// with its captures.
    pub kind: Option<FnKind>,
    /// What it captures, in order: each variable's name, and the type of
    /// the field it's in, a reference to it or its value.
    pub fields: Vec<(String, Type)>,
}

impl ClosureDef {
    /// Its fields' types at type `t`, which is it.
    pub fn fields_at(&self, t: &Type) -> Vec<Type> {
        let s = self.subst(t);
        self.fields.iter().map(|(_, f)| s.apply(f)).collect()
    }

    /// Its signature at type `t`, which is it.
    pub fn sig_at(&self, t: &Type) -> Type {
        self.subst(t).apply(&Type::Fn(self.sig.clone()))
    }

    fn subst(&self, t: &Type) -> Subst {
        let Type::Adt(_, regions, args) = t else {
            unreachable!("a closure type is a struct's")
        };
        let s = Subst::new(&self.params, args);
        match regions.len() == self.lifetimes.len() {
            true => s.with_regions(&self.lifetimes, regions),
            false => s,
        }
    }
}

/// What an opaque type, `impl Fn(i64) -> i64` in a function's return type,
/// is (see `Type::Opaque`). Its type is generic over every lifetime and type
/// parameter of the function, as the hidden type it stands for may mention
/// any of them; it's *invariant* in each, as how the hidden type uses them
/// isn't for the rest of the program to know.
///
/// Everywhere but in the function, it's like a type parameter whose bounds
/// are its `preds` and `eqs`: it implements what they say, and the impls
/// that apply to any type that does (blanket impls), and nothing else. Its
/// hidden type was checked to implement what they say, under the
/// function's bounds, so monomorphisation, which replaces it with that,
/// finds the same impls: by coherence, a blanket impl that applies to it is
/// the only impl of its trait.
#[derive(Debug, Clone)]
pub struct OpaqueDef {
    /// The function it's the return type of, by symbol, and by name.
    pub owner: String,
    pub owner_name: String,
    /// The function's type and lifetime parameters, the elided ones too.
    pub params: Vec<String>,
    pub lifetimes: Vec<String>,
    /// What it implements, on itself as `generic` or on projections of
    /// that, mentioning `params` and `lifetimes`.
    pub preds: Vec<Pred>,
    /// What its equality bounds say, like `(<Self as Iter>::Item, i64)`,
    /// with `generic` for `Self`.
    pub eqs: Vec<(Type, Type)>,
    /// The type it stands for, mentioning `params`, once the function's
    /// body is checked. Its regions don't matter: monomorphisation
    /// ignores them, and the borrow checker of the function's body makes
    /// its own.
    pub hidden: Option<Type>,
    /// Where it's written.
    pub span: Span,
}

impl OpaqueDef {
    /// Its type with the function's parameters as arguments, as the
    /// function sees it.
    pub fn generic(&self, symbol: &str) -> Type {
        Type::Opaque(
            symbol.to_string(),
            self.lifetimes.iter().cloned().map(Region::Named).collect(),
            self.params.iter().cloned().map(Type::Param).collect(),
        )
    }

    /// What its parameters are at type `t`, which is it.
    pub fn subst(&self, t: &Type) -> Subst {
        let Type::Opaque(_, regions, args) = t else {
            unreachable!("`{t}` isn't an opaque type")
        };
        let s = Subst::new(&self.params, args);
        match regions.len() == self.lifetimes.len() {
            true => s.with_regions(&self.lifetimes, regions),
            false => s,
        }
    }

    /// Its hidden type at type `t`, which is it.
    pub fn hidden_at(&self, t: &Type) -> Type {
        let hidden = self
            .hidden
            .as_ref()
            .expect("the hidden type is known by now");
        self.subst(t).apply(hidden)
    }
}

/// A trait.
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Where its name is declared, unless it's built in.
    pub span: Option<Span>,
    /// Its supertraits, as written.
    pub supers: Vec<String>,
    /// Its associated types.
    pub assoc: Vec<AssocDecl>,
    pub methods: Vec<MethodDef>,
}

/// An associated type of a trait: `type Item: Discard;`.
#[derive(Debug, Clone)]
pub struct AssocDecl {
    pub name: String,
    /// The traits every impl's type for it must implement, as written:
    /// generic code may assume it does.
    pub bounds: Vec<String>,
    pub span: Span,
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
    /// Where its effects are, or would be, written, `with ..`, with each
    /// effect as written, unless it's built in: for a fix that adds more.
    pub effects_at: Option<(Span, Vec<String>)>,
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
    /// What its equality bounds say, like `(T::Item, i64)`.
    pub eqs: Vec<(Type, Type)>,
    /// What it says each of the trait's associated types is, by name, as
    /// written, mentioning `params` and `lifetimes`.
    pub assoc: Vec<(String, Type)>,
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
    /// Every closure that captures, by its symbol.
    pub closures: HashMap<String, ClosureDef>,
    /// Every opaque type, by its symbol.
    pub opaques: HashMap<String, OpaqueDef>,
}

/// How a type implements a trait.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// It's built into the language, like `i64: Eq`.
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
                assoc: Vec::new(),
                methods: Vec::new(),
            },
        );
        // trait Copy: Discard {}
        traits.insert(
            "Copy".to_string(),
            TraitDef {
                span: None,
                supers: vec!["Discard".to_string()],
                assoc: Vec::new(),
                methods: Vec::new(),
            },
        );
        // trait Eq { fn eq(&self, other: &Self) -> bool; }
        let this = || Type::Ref(Region::Named("'_0".into()), false, Box::new(param("Self")));
        let other = Type::Ref(Region::Named("'_1".into()), false, Box::new(param("Self")));
        let eq = MethodDef {
            name: "eq".into(),
            symbol: "<Eq>::eq".into(),
            effects_at: None,
            scheme: Scheme {
                params: vec!["Self".into()],
                lifetimes: vec!["'_0".into(), "'_1".into()],
                bounds: Vec::new(),
                preds: vec![Pred {
                    ty: param("Self"),
                    trait_name: "Eq".into(),
                }],
                eqs: Vec::new(),
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
                assoc: Vec::new(),
                methods: vec![eq],
            },
        );
        // trait Int: Copy + Eq {}
        //
        // Sealed: no impl can be written, so the integer types are all
        // there ever are, and generic code over them may count on what
        // they all have in common (see docs/11-integers.md).
        traits.insert(
            "Int".to_string(),
            TraitDef {
                span: None,
                supers: vec!["Copy".to_string(), "Eq".to_string()],
                assoc: Vec::new(),
                methods: Vec::new(),
            },
        );
        // trait Signed: Int {}
        // trait Unsigned: Int {}
        //
        // `Int`'s two halves, sealed too: of a `T: Signed`, the least
        // value is -1 minus the greatest, and of a `T: Unsigned` it's 0
        // (see docs/12-bits.md).
        for name in ["Signed", "Unsigned"] {
            traits.insert(
                name.to_string(),
                TraitDef {
                    span: None,
                    supers: vec!["Int".to_string()],
                    assoc: Vec::new(),
                    methods: Vec::new(),
                },
            );
        }
        // trait FnOnce { type (signature); }
        // trait FnMut: FnOnce {}
        // trait Fn: FnMut {}
        //
        // Calling isn't a method, as a call takes any number of arguments.
        for (name, supers) in [
            ("FnOnce", None),
            ("FnMut", Some("FnOnce")),
            ("Fn", Some("FnMut")),
        ] {
            let assoc = match supers {
                None => vec![AssocDecl {
                    name: SIG.into(),
                    bounds: Vec::new(),
                    span: Span::new(0, 0),
                }],
                Some(_) => Vec::new(),
            };
            traits.insert(
                name.to_string(),
                TraitDef {
                    span: None,
                    supers: supers.into_iter().map(str::to_string).collect(),
                    assoc,
                    methods: Vec::new(),
                },
            );
        }
        Traits {
            traits,
            impls: Vec::new(),
            closures: HashMap::new(),
            opaques: HashMap::new(),
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

    /// The trait among `tr` and its supertraits with an associated type
    /// `name`, if exactly one has: `T::Item` for `T: tr` means it's that
    /// one's. More than one, all of them, for saying so.
    pub fn assoc_owner(&self, tr: &str, name: &str) -> Vec<String> {
        self.implied(tr)
            .into_iter()
            .filter(|t| self.traits[t].assoc.iter().any(|a| a.name == name))
            .collect()
    }

    /// The method `name` of trait `tr`, if it has one.
    pub fn method(&self, tr: &str, name: &str) -> Option<&MethodDef> {
        self.traits.get(tr)?.methods.iter().find(|m| m.name == name)
    }

    /// The impl of `tr` (or an inherent one, for `None`) whose type matches
    /// `ty`, with what its type parameters are for it to. Coherence makes
    /// that at most one.
    pub fn select(
        &self,
        ty: &Type,
        tr: Option<&str>,
        local: &[(Type, Type)],
    ) -> Option<(usize, Vec<Type>)> {
        self.impls.iter().enumerate().find_map(|(i, d)| {
            if d.trait_name.as_deref() != tr {
                return None;
            }
            let mut found = HashMap::new();
            if !matches(&d.self_ty, ty, &d.params, &mut found) {
                return None;
            }
            Some((i, impl_args(d, &found, &self.impls, &self.anon(), local)?))
        })
    }

    /// Does `ty` implement `tr`, given that the bounds `env` hold? `ty`
    /// must have no type variables in it. `env` is bounds on type
    /// parameters and projections, and, in an impl's method with a `where`
    /// clause, on the impl's type: those hold there even if they're false,
    /// as no call of the method can then prove them, and it never runs, so
    /// monomorphisation never asks.
    ///
    /// Monomorphisation asks it again, with no bounds, of the types each
    /// copy is at (`mono::Cx::target`), and expects the same answer as the
    /// checker got with bounds. That holds as every obligation was proved
    /// with the bounds in scope, and every impl, derived ones too, needs
    /// exactly its `preds`: a derived impl that needed less than its
    /// fields do (`Checker::derive`) would pass here and fail there. One
    /// whose bounds are written, `#[derive(Discard where T: Discard)]`,
    /// needs those, which were checked to give what its fields need. Which
    /// module anything is in doesn't matter here: modules decide which
    /// names are in scope, never which impl applies (lemma M3 of
    /// docs/06d-modules.md). If it doesn't, the obligation
    /// that fails, at the bottom of the search: for `Option<fn()>: Eq`,
    /// `fn(): Eq`.
    ///
    /// A rigid projection, like `T::Item`, implements what a bound in
    /// `env` says it does, and what the trait declares of it: `type Item:
    /// Discard;` says every impl's `Item` does, which each impl was checked
    /// for (`Checker::assoc_bounds`). Monomorphisation, where it's what it
    /// stands for, gets the same answer: a bound on it was proved of that
    /// by each caller, and the declaration of every impl's. `eqs` are the
    /// equality bounds in scope, for an impl that has some of its own,
    /// which hold by normalising, the same before and after substitution
    /// (see `Projections`). `ty` must be normalised as far as it goes.
    ///
    /// An opaque type implements what its bounds say, and what blanket
    /// impls give that, as a rigid projection does; monomorphisation asks
    /// of its hidden type, which was checked to implement its bounds with
    /// the bounds of the function it's from, and gets the same answer (see
    /// `matches`).
    pub fn implements(
        &self,
        ty: &Type,
        tr: &str,
        env: &[Pred],
        eqs: &[(Type, Type)],
    ) -> Result<Source, Pred> {
        self.implements_at(ty, tr, env, eqs, 0)
    }

    fn implements_at(
        &self,
        ty: &Type,
        tr: &str,
        env: &[Pred],
        eqs: &[(Type, Type)],
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
                self.implements_at(t, tr, env, eqs, depth + 1)?;
            }
            Ok(Source::Builtin)
        };
        // The built-in impls. Plain values may be thrown away, and so may
        // references, whatever they point to: dropping one doesn't drop
        // that. A box holds what it points to, so it may be if that may.
        match (tr, ty) {
            (
                "Discard",
                Type::Int(_)
                | Type::Bool
                | Type::Char
                | Type::Unit
                | Type::Never
                | Type::Fn(_)
                | Type::Ref(..),
            ) => return Ok(Source::Builtin),
            ("Discard", Type::Box(inner)) => return all(&[inner]),
            (
                "Copy",
                Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never | Type::Fn(_),
            ) => {
                return Ok(Source::Builtin);
            }
            ("Copy", Type::Ref(_, false, _)) => return Ok(Source::Builtin),
            ("Copy", Type::Ref(_, true, _) | Type::Box(_)) => return Err(fail()),
            ("Eq", Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never) => {
                return Ok(Source::Builtin);
            }
            // A string compares by its bytes, and a slice value by value,
            // if its values compare: the same length, and equal values at
            // each index.
            ("Eq", Type::Str) => return Ok(Source::Builtin),
            ("Eq", Type::Slice(inner)) => return all(&[inner]),
            ("Int", Type::Int(_)) => return Ok(Source::Builtin),
            ("Signed", Type::Int(t)) if t.signed() => return Ok(Source::Builtin),
            ("Unsigned", Type::Int(t)) if !t.signed() => return Ok(Source::Builtin),
            ("Signed" | "Unsigned", Type::Int(_)) => return Err(fail()),
            // References and boxes compare what they point to.
            ("Eq", Type::Ref(_, _, inner) | Type::Box(inner)) => return all(&[inner]),
            ("Eq", Type::Fn(_)) => return Err(fail()),
            // Functions can be called any number of times. A reference to
            // what can be called through a shared reference can be too, and
            // one to what can be through a mutable one, as long as it's
            // mutable: calling `&F` calls `F` by `&`.
            ("Fn" | "FnMut" | "FnOnce", Type::Fn(_)) => return Ok(Source::Builtin),
            ("Fn" | "FnMut" | "FnOnce", Type::Ref(_, false, inner)) => {
                self.implements_at(inner, "Fn", env, eqs, depth + 1)?;
                return Ok(Source::Builtin);
            }
            ("FnMut" | "FnOnce", Type::Ref(_, true, inner)) => {
                self.implements_at(inner, "FnMut", env, eqs, depth + 1)?;
                return Ok(Source::Builtin);
            }
            ("Fn", Type::Ref(_, true, _)) => return Err(fail()),
            _ => {}
        }
        // A closure is what it captures: it may be thrown away, or copied,
        // if all of that may be. A reference it holds always may be thrown
        // away, and copied unless it's `&mut`.
        if let Type::Adt(symbol, ..) = ty
            && let Some(c) = self.closures.get(symbol)
        {
            let Some(kind) = c.kind else {
                unreachable!("`{ty}: {tr}` asked before the closure's captures were known")
            };
            return match (tr, FnKind::of(tr)) {
                ("Discard" | "Copy", _) => all(&c.fields_at(ty).iter().collect::<Vec<_>>()),
                (_, Some(wanted)) if kind <= wanted => Ok(Source::Builtin),
                _ => Err(fail()),
            };
        }
        // What an opaque type's bounds say it implements, it does; or it
        // might by a blanket impl, below. An opaque type has no impls of
        // its own: nothing can name it. So does a projection on it that
        // they say does.
        if let Some(Source::Assumed) = self.opaque_implements(ty, tr) {
            return Ok(Source::Assumed);
        }
        if let Type::Param(_) | Type::Proj(..) = ty {
            let implied = |t: &str| self.implied(t).iter().any(|t| t == tr);
            let assumed = env.iter().any(|p| p.ty == *ty && implied(&p.trait_name));
            let declared = match ty {
                Type::Proj(_, of, name) => self.traits[of]
                    .assoc
                    .iter()
                    .find(|a| a.name == *name)
                    .is_some_and(|a| a.bounds.iter().any(|b| implied(b))),
                _ => false,
            };
            return match assumed || declared {
                true => Ok(Source::Assumed),
                false => Err(fail()),
            };
        }
        // A bound on a type that isn't a parameter, from an impl's method's
        // `where` clause at the impl's type, holds in the method: if it's
        // false, no call can prove it, and the method never runs.
        if env
            .iter()
            .any(|p| p.ty == *ty && self.implied(&p.trait_name).iter().any(|t| t == tr))
        {
            return Ok(Source::Assumed);
        }
        let Some((i, args)) = self.select(ty, Some(tr), eqs) else {
            return Err(fail());
        };
        let d = &self.impls[i];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            let t = normalise(&self.impls, &self.anon(), eqs, &s.apply(&p.ty));
            self.implements_at(&t, &p.trait_name, env, eqs, depth + 1)?;
        }
        for (lhs, rhs) in &d.eqs {
            let (l, r) = (s.apply(lhs), s.apply(rhs));
            let normal = |t: &Type| normalise(&self.impls, &self.anon(), eqs, t).erased();
            if !fits(&normal(&l), &normal(&r), is_sig(lhs)) {
                return Err(fail());
            }
        }
        Ok(Source::Impl(i, args))
    }

    /// `Some(Assumed)` if `ty` is an opaque type, or a projection on one,
    /// that the opaque type's bounds say implements `tr`.
    fn opaque_implements(&self, ty: &Type, tr: &str) -> Option<Source> {
        let mut root = ty;
        while let Type::Proj(inner, ..) = root {
            root = inner;
        }
        let Type::Opaque(symbol, ..) = root else {
            return None;
        };
        let d = self.opaques.get(symbol)?;
        let s = d.subst(root);
        let erased = ty.erased();
        d.preds
            .iter()
            .any(|p| {
                s.apply(&p.ty).erased() == erased
                    && self.implied(&p.trait_name).iter().any(|t| t == tr)
            })
            .then_some(Source::Assumed)
    }

    /// The closures and opaque types, for normalising.
    pub fn anon(&self) -> Anon<'_> {
        Anon {
            closures: &self.closures,
            opaques: &self.opaques,
        }
    }
}

/// Is `lhs`, of an equality bound, what an `Fn` bound says calls take and
/// give, `<F as FnOnce>::(signature)`?
pub fn is_sig(lhs: &Type) -> bool {
    matches!(lhs, Type::Proj(_, _, n) if n == SIG)
}

/// Does `t` meet an equality bound that says it's `want`? For an `Fn`
/// bound's signature (`sig`), the effects `want` says are the most its
/// calls may have, so a function that has fewer meets it: it can be
/// called wherever one that may do more can. Otherwise only `want`
/// itself does.
pub fn fits(t: &Type, want: &Type, sig: bool) -> bool {
    match (t, want) {
        (Type::Fn(f), Type::Fn(g)) if sig => {
            f.with_effects(Type::no_effects()) == g.with_effects(Type::no_effects())
                && subset(&f.effects, &g.effects)
        }
        _ => t == want,
    }
}

/// Is every effect of `a` one of `b`'s?
pub fn subset(a: &Type, b: &Type) -> bool {
    a.effect_items()
        .iter()
        .all(|e| b.effect_items().contains(e))
}

/// The arguments of impl `d`, at a type its header matched with `found`,
/// or `None` if a parameter its type doesn't have can't be decided there.
///
/// Each type parameter in the header is from the match. One that isn't is
/// *decided* by an equality bound (`decider`): it's the part of the
/// bound's type where the parameter is, in what the bound's projection is
/// at the matched type, as `U` is `i64` in `impl<U, F: Fn() -> U> Get for
/// Thunk<F>` at `Thunk<fn() -> i64>`. That's a function of the type too,
/// as normalising is, and none if the projection's still rigid there, or
/// what it is has no such part, or the part has a region of a function
/// type it's in (`'^0` of `fn(&i64) -> &i64` is no region outside it).
///
/// An effect parameter's needn't be in the header, as in `impl<f, F: Fn()
/// with f> Tr for W<F>`: then it's the least that makes the impl's `Fn`
/// bounds hold, the effects of what's called, less the others a bound
/// allows. Effects form a join-semilattice, so the least always exists,
/// and depends only on the type matched.
///
/// So an impl found for a type has one meaning there, as for any of its
/// parameters (coherence), and the arguments commute with substitution.
fn impl_args(
    d: &ImplDef,
    found: &HashMap<String, Type>,
    impls: &[ImplDef],
    anon: &Anon,
    local: &[(Type, Type)],
) -> Option<Vec<Type>> {
    let unfound = |p: &String| match crate::parser::is_effect_name(p) {
        true => Type::no_effects(),
        false => Type::Unit,
    };
    let mut args: Vec<Type> = d
        .params
        .iter()
        .map(|p| found.get(p).cloned().unwrap_or_else(|| unfound(p)))
        .collect();
    // The header's parameters are all known, so each bound that decides
    // one can be worked out now: its projection is on one of them.
    let s = Subst::new(&d.params, &args);
    for (i, p) in d.params.iter().enumerate() {
        if found.contains_key(p) || crate::parser::is_effect_name(p) {
            continue;
        }
        let k = decider(&d.self_ty, &d.params, &d.eqs, p)?;
        let (lhs, rhs) = &d.eqs[k];
        let at = normalise(impls, anon, local, &s.apply(lhs));
        let value = part_at(rhs, &at, p, is_sig(lhs))?;
        let bound = |r: &&Region| matches!(r, Region::Named(n) if n.starts_with("'^"));
        if value.regions().iter().any(bound) {
            return None;
        }
        args[i] = value;
    }
    let s = Subst::new(&d.params, &args);
    for (i, p) in d.params.iter().enumerate() {
        if found.contains_key(p) || !crate::parser::is_effect_name(p) {
            continue;
        }
        let me = Type::Param(p.clone());
        let mut items = Vec::new();
        for (lhs, rhs) in &d.eqs {
            let Type::Fn(want) = rhs else { continue };
            if !is_sig(lhs) || !want.effects.effect_items().contains(&me) {
                continue;
            }
            let others: Vec<Type> = want
                .effects
                .effect_items()
                .iter()
                .filter(|e| **e != me)
                .map(|e| s.apply(e))
                .collect();
            let others = Type::effects(others);
            if let Type::Fn(sig) = normalise(impls, anon, local, &s.apply(lhs)) {
                let more = sig.effects.effect_items().iter();
                items.extend(more.filter(|e| !others.effect_items().contains(e)).cloned());
            }
        }
        args[i] = Type::effects(items);
    }
    Some(args)
}

/// Which of equality bounds `eqs` decides type parameter `p` of an impl
/// for `self_ty` with parameters `params`, which `self_ty` doesn't have,
/// if one does: the first whose projection is on a parameter `self_ty`
/// has outside function types (where matching it decides it as a type
/// that means something anywhere), with `p` where matching can find it
/// (`decidable`). Not one on a parameter decided itself: that might not
/// end, and isn't needed yet.
pub fn decider(self_ty: &Type, params: &[String], eqs: &[(Type, Type)], p: &str) -> Option<usize> {
    let header = outside_fns(self_ty);
    eqs.iter().position(|(lhs, rhs)| {
        matches!(lhs, Type::Proj(s, ..)
            if matches!(&**s, Type::Param(q) if params.contains(q) && header.contains(q)))
            && decidable(rhs, p, is_sig(lhs))
    })
}

/// The type parameters `t` has outside function types.
pub fn outside_fns(t: &Type) -> Vec<String> {
    match t {
        Type::Fn(_) => Vec::new(),
        Type::Param(p) => vec![p.clone()],
        t => t.parts().flat_map(outside_fns).collect(),
    }
}

/// Is type parameter `p` in `pat` where matching `pat` to a type finds
/// what it is? Not inside a projection, which says nothing of what it's
/// on, nor in effects, which bounds give the most of, nor inside a
/// function type, but for a signature's (`sig`) parameters and result.
fn decidable(pat: &Type, p: &str, sig: bool) -> bool {
    match pat {
        Type::Param(q) => q == p,
        Type::Proj(..) | Type::Effects(_) => false,
        Type::Fn(f) if sig => f
            .params
            .iter()
            .chain([&*f.ret])
            .any(|t| decidable(t, p, false)),
        Type::Fn(_) => false,
        t => t.parts().any(|t| decidable(t, p, false)),
    }
}

/// The part of `t` where `pat` has type parameter `p`, if `t` has the
/// shape `pat` has on the way there (see `decidable`).
fn part_at(pat: &Type, t: &Type, p: &str, sig: bool) -> Option<Type> {
    match (pat, t) {
        (Type::Param(q), t) if q == p => Some(t.clone()),
        (Type::Proj(..) | Type::Effects(_), _) => None,
        (Type::Fn(f), Type::Fn(g)) if sig && pat.same_shape(t) => {
            let pairs = f.params.iter().zip(&g.params).chain([(&*f.ret, &*g.ret)]);
            pairs.into_iter().find_map(|(a, b)| part_at(a, b, p, false))
        }
        (Type::Fn(_), _) => None,
        (pat, t) if pat.same_shape(t) => pat
            .parts()
            .zip(t.parts())
            .find_map(|(a, b)| part_at(a, b, p, false)),
        _ => None,
    }
}

/// The types with no names a program can write: closures' and opaque
/// types. What a projection on one is is in their definitions rather than
/// impls.
#[derive(Clone, Copy)]
pub struct Anon<'a> {
    pub closures: &'a HashMap<String, ClosureDef>,
    pub opaques: &'a HashMap<String, OpaqueDef>,
}

/// What the projections in types stand for: the associated types of every
/// impl, and what the equality bounds in scope say.
///
/// Working one out is *normalising* it, and it's a function of the type
/// it's on, regions and all, that commutes with substitution, which is what
/// makes it sound for generic code, the borrow checker and
/// monomorphisation to each do it at their own time (see
/// docs/06b-associated-types.md):
///
/// - At most one impl matches a type (coherence), and only by *matching*
///   its header to the type, never unifying: more known of a type can't
///   pick another impl, or none.
/// - The header is parametric (`check::traits::collect`'s `parametric`): each
///   lifetime and type parameter once, no `'static`. So matching gives each
///   of them exactly one region or type, with the regions where the type
///   has them, and the associated type is what the impl says with those.
///   It may only mention parameters that are in the header outside every
///   function type (`check::traits::collect`'s `constrained`): matching doesn't
///   decide those, or decides them as regions that only mean something
///   inside a function type. Or type parameters an equality bound decides,
///   from a projection on a parameter in the header outside function types
///   (`decider`), as the part of what that is where the bound says, with
///   no region of a function type it's in; or, for an associated effect,
///   effect parameters an `Fn` bound decides (`decided_effects`), as the
///   least effects that make it hold at the type matched (`impl_args`):
///   functions of that type too, so substitution still commutes. The
///   projections that decide them end as an associated type's do.
/// - It's never cached by type with regions erased: `<&'x i64 as Tr>::Out`
///   and `<&'y i64 as Tr>::Out` are different types.
/// - It ends: the projections in an impl's associated type are on smaller
///   types than its header, or on the same type parameter, for a blanket
///   impl, to a trait that can't lead back (`Checker::assoc_termination`).
#[derive(Debug, Clone, Default)]
pub struct Projections {
    /// Every trait impl.
    impls: Arc<Vec<ImplDef>>,
    /// Every closure that captures, by symbol: what it takes and gives is
    /// what `<C as FnOnce>::(signature)` is.
    pub closures: HashMap<String, ClosureDef>,
    /// Every opaque type, by symbol: what its equality bounds say a
    /// projection on it is.
    pub opaques: HashMap<String, OpaqueDef>,
    /// What the equality bounds in scope say: `(T::Item, i64)` for
    /// `T: Iter<Item = i64>`.
    pub local: Vec<(Type, Type)>,
}

impl Projections {
    pub fn new(traits: &Traits) -> Projections {
        let impls = traits.impls.iter().filter(|d| d.trait_name.is_some());
        Projections {
            impls: Arc::new(impls.cloned().collect()),
            closures: traits.closures.clone(),
            opaques: traits.opaques.clone(),
            local: Vec::new(),
        }
    }

    fn anon(&self) -> Anon<'_> {
        Anon {
            closures: &self.closures,
            opaques: &self.opaques,
        }
    }

    /// `t`, with every projection in it that can be worked out, worked out.
    pub fn normalise(&self, t: &Type) -> Type {
        normalise(&self.impls, &self.anon(), &self.local, t)
    }

    /// `t`, with every opaque type in it replaced by the type it stands
    /// for, as far down as that goes, and then normalised: for
    /// monomorphisation, once every function's body is checked. It ends,
    /// as no hidden type holds its own opaque type, through others or not
    /// (`Checker::opaque_cycles`).
    pub fn reveal(&self, t: &Type) -> Type {
        let t = t.map(|p| self.reveal(p));
        let t = match &t {
            Type::Opaque(symbol, ..) => self.reveal(&self.opaques[symbol].hidden_at(&t)),
            _ => t,
        };
        self.normalise(&t)
    }
}

/// `t`, with every projection in it worked out that can be: by the impl of
/// `impls` its type matches, or what `local` says of a rigid one.
pub fn normalise(impls: &[ImplDef], anon: &Anon, local: &[(Type, Type)], t: &Type) -> Type {
    let t = t.map(|p| normalise(impls, anon, local, p));
    let Type::Proj(s, tr, name) = &t else {
        return t;
    };
    if let Some((_, u)) = local.iter().find(|(p, _)| *p == t) {
        return u.clone();
    }
    match project(impls, anon, local, s, tr, name) {
        Some(u) => normalise(impls, anon, local, &u),
        None => t,
    }
}

/// What `<s as tr>::name` is by the impl of `impls` that's for `s`, if one
/// is: the type it says, with the parts of `s` its parameters match, and
/// the regions of `s` where its lifetimes are.
pub fn project(
    impls: &[ImplDef],
    anon: &Anon,
    local: &[(Type, Type)],
    s: &Type,
    tr: &str,
    name: &str,
) -> Option<Type> {
    if let Type::Param(_) | Type::Var(_) | Type::Proj(..) = s {
        return None;
    }
    // What an opaque type's equality bounds say; or else what a blanket
    // impl does, below, or nothing: then it's rigid.
    if let Type::Opaque(symbol, ..) = s
        && let Some(d) = anon.opaques.get(symbol)
    {
        let sub = d.subst(s);
        let proj = Type::Proj(Box::new(s.clone()), tr.to_string(), name.to_string()).erased();
        if let Some((_, rhs)) = d
            .eqs
            .iter()
            .find(|(lhs, _)| sub.apply(lhs).erased() == proj)
        {
            return Some(sub.apply(rhs));
        }
    }
    // What a call of a function, a closure, or a reference to one takes
    // and gives: its signature, with the closure's regions where its
    // lifetimes are.
    if name == SIG {
        return match s {
            Type::Fn(_) => Some(s.clone()),
            Type::Adt(symbol, ..) => Some(anon.closures.get(symbol)?.sig_at(s)),
            Type::Ref(_, _, inner) => match &**inner {
                Type::Param(_) | Type::Var(_) | Type::Proj(..) => {
                    Some(Type::Proj(inner.clone(), "FnOnce".into(), SIG.into()))
                }
                inner => project(impls, anon, local, inner, tr, name),
            },
            _ => None,
        };
    }
    for d in impls.iter().filter(|d| d.trait_name.as_deref() == Some(tr)) {
        let mut found = HashMap::new();
        if !matches(&d.self_ty, s, &d.params, &mut found) {
            continue;
        }
        let mut regions = HashMap::new();
        regions_at(&d.self_ty, s, &d.lifetimes, &mut regions);
        let (_, rhs) = d.assoc.iter().find(|(n, _)| n == name)?;
        let args = impl_args(d, &found, impls, anon, local)?;
        let regions: Vec<Region> = d
            .lifetimes
            .iter()
            .map(|l| regions.get(l).cloned().unwrap_or(Region::Erased))
            .collect();
        let s = Subst::new(&d.params, &args).with_regions(&d.lifetimes, &regions);
        return Some(s.apply(rhs));
    }
    None
}

/// The region `t` has where `pat`, which `t` matches, has each of
/// `lifetimes`, outside function types: a function type's regions may be
/// those it's generic over, which mean nothing outside it.
fn regions_at(pat: &Type, t: &Type, lifetimes: &[String], out: &mut HashMap<String, Region>) {
    let mut at = |p: &Region, r: &Region| {
        if let Region::Named(n) = p
            && lifetimes.contains(n)
        {
            out.insert(n.clone(), r.clone());
        }
    };
    match (pat, t) {
        (Type::Fn(_), _) => {}
        (Type::Ref(p, _, a), Type::Ref(r, _, b)) => {
            at(p, r);
            regions_at(a, b, lifetimes, out);
        }
        (Type::Adt(_, ps, xs), Type::Adt(_, rs, ys)) => {
            for (p, r) in ps.iter().zip(rs) {
                at(p, r);
            }
            for (x, y) in xs.iter().zip(ys) {
                regions_at(x, y, lifetimes, out);
            }
        }
        (Type::Box(a), Type::Box(b)) | (Type::Slice(a), Type::Slice(b)) => {
            regions_at(a, b, lifetimes, out)
        }
        _ => {}
    }
}

/// What `==` on values of type `t` compares: what any references and boxes
/// point to, as their built-in impls of `Eq` do.
pub(crate) fn eq_subject(t: &Type) -> &Type {
    match t {
        Type::Ref(_, _, inner) | Type::Box(inner) => eq_subject(inner),
        t => t,
    }
}

fn param(name: &str) -> Type {
    Type::Param(name.to_string())
}

/// Does `pat`, a type mentioning type parameters `params`, match `t` for
/// some choice of them? Adds what they must be to `found`. Regions don't
/// matter: an impl is for a type at every lifetime.
///
/// Invariants this relies on, which anything that lets resolution decide
/// more (associated types, say) must keep or replace:
///
/// - Every trait impl is *parametric* in lifetimes (`check::traits::collect`'s
///   `parametric`): a lifetime or type parameter appears once in its type,
///   never `'static`, with no lifetime bounds. So the impl found for
///   `Pair<'x, 'y>` was checked for every two lifetimes, and what it gives
///   can't depend on which. An associated type is a type it gives:
///   `<Pair<'x, 'y> as Tr>::Out` is worked out with `'x` and `'y`, each
///   where the header has it (`project`), never with regions erased, and
///   only from what's outside function types.
/// - Nor does it look at what a function type is generic over: an impl for
///   `fn(&i64) -> &i64`, for every lifetime, is found for a `fn(&'a i64) ->
///   &'a i64`. That's sound only because every function value works at
///   every lifetime, and because an associated type can't mention a
///   parameter that's only inside a function type. A function value holds
///   no references: a closure that captures anything has a type of its own
///   (`ClosureDef`), never a function type, and the checker rejects one
///   where a function type is wanted. Anything that lets a function type
///   hold what it borrows must replace this.
/// - A closure's type is a struct's, parametric in its lifetimes like any,
///   and nothing can name it in an impl's header, so only blanket impls and
///   the built-in ones are found for it. What those need of its signature,
///   `F: Fn(&T) -> bool`, is an equality bound on its `SIG`, which
///   `implements_at` compares with what's bound and all, but for its
///   effects, of which the bound says the most (`fits`): a signature
///   generic over a lifetime isn't one at a lifetime fixed outside. That
///   compares names, so bound lifetimes must be named `'^0`, `'^1`, .. in
///   the order they're first written, wherever a function type is made
///   (`generics::region`, `Lifetimes::ClosureParams`).
/// - Matching decides only the parameters in the header. Those a bound
///   decides (`decider`) are worked out from what it matched, never by
///   matching: so they never make an impl apply to more or fewer types,
///   and overlap is still unifying headers (`overlap`). What they're
///   decided as is a function of the type, with no lifetime of a function
///   type in it (`impl_args`).
/// - Nor can anything name an opaque type, so only blanket impls are found
///   for it, and only by its bounds. Its hidden type implements those
///   bounds (`Checker::hide`), so the same blanket impl is found for that,
///   and, by coherence, no other impl of the trait exists: monomorphisation,
///   which only sees the hidden type, finds the impl the checker did.
/// - A parameter never matches a slice, `[T]`, or `str`, as no type
///   parameter stands for one (`Type::Slice`): an impl's methods could have its `T`
///   by value. `overlap` agrees, so `impl<T> Tr for &T` and `impl Tr for
///   &[i64]` don't overlap, and the checker and monomorphisation find the
///   same one.
pub fn matches(pat: &Type, t: &Type, params: &[String], found: &mut HashMap<String, Type>) -> bool {
    match pat {
        // A parameter never stands for a slice or `str` (see `Type::Slice`).
        Type::Param(p) if params.contains(p) => match found.get(p) {
            Some(prev) => prev.erased() == t.erased(),
            None if t.is_unsized() => false,
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
            if occurs(p, t, s) || t.is_unsized() {
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
