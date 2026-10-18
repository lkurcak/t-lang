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
    /// What its equality bounds say, like `(T::Item, int)`.
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
    /// must have no type variables in it, and `env` is bounds on type
    /// parameters only.
    ///
    /// Monomorphisation asks it again, with no bounds, of the types each
    /// copy is at (`mono::Cx::target`), and expects the same answer as the
    /// checker got with bounds. That holds as every obligation was proved
    /// with the bounds in scope, and every impl, derived ones too, needs
    /// exactly its `preds`: a derived impl that needed less than its
    /// fields do (`Checker::derive`) would pass here and fail there. If it
    /// doesn't, the obligation
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
        let Some((i, args)) = self.select(ty, Some(tr)) else {
            return Err(fail());
        };
        let d = &self.impls[i];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            let t = normalise(&self.impls, eqs, &s.apply(&p.ty));
            self.implements_at(&t, &p.trait_name, env, eqs, depth + 1)?;
        }
        for (lhs, rhs) in &d.eqs {
            let (l, r) = (s.apply(lhs), s.apply(rhs));
            if normalise(&self.impls, eqs, &l).erased() != normalise(&self.impls, eqs, &r).erased()
            {
                return Err(fail());
            }
        }
        Ok(Source::Impl(i, args))
    }
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
/// - The header is parametric (`check::traits`'s `parametric`): each
///   lifetime and type parameter once, no `'static`. So matching gives each
///   of them exactly one region or type, with the regions where the type
///   has them, and the associated type is what the impl says with those.
///   It may only mention parameters that are in the header outside every
///   function type (`check::traits`'s `constrained`): matching doesn't
///   decide those, or decides them as regions that only mean something
///   inside a function type.
/// - It's never cached by type with regions erased: `<&'x int as Tr>::Out`
///   and `<&'y int as Tr>::Out` are different types.
/// - It ends: the projections in an impl's associated type are on smaller
///   types than its header, or on the same type parameter, for a blanket
///   impl, to a trait that can't lead back (`Checker::assoc_termination`).
#[derive(Debug, Clone, Default)]
pub struct Projections {
    /// Every trait impl.
    impls: Arc<Vec<ImplDef>>,
    /// What the equality bounds in scope say: `(T::Item, int)` for
    /// `T: Iter<Item = int>`.
    pub local: Vec<(Type, Type)>,
}

impl Projections {
    pub fn new(traits: &Traits) -> Projections {
        let impls = traits.impls.iter().filter(|d| d.trait_name.is_some());
        Projections {
            impls: Arc::new(impls.cloned().collect()),
            local: Vec::new(),
        }
    }

    /// `t`, with every projection in it that can be worked out, worked out.
    pub fn normalise(&self, t: &Type) -> Type {
        normalise(&self.impls, &self.local, t)
    }
}

/// `t`, with every projection in it worked out that can be: by the impl of
/// `impls` its type matches, or what `local` says of a rigid one.
pub fn normalise(impls: &[ImplDef], local: &[(Type, Type)], t: &Type) -> Type {
    let t = t.map(|p| normalise(impls, local, p));
    let Type::Proj(s, tr, name) = &t else {
        return t;
    };
    if let Some((_, u)) = local.iter().find(|(p, _)| *p == t) {
        return u.clone();
    }
    match project(impls, s, tr, name) {
        Some(u) => normalise(impls, local, &u),
        None => t,
    }
}

/// What `<s as tr>::name` is by the impl of `impls` that's for `s`, if one
/// is: the type it says, with the parts of `s` its parameters match, and
/// the regions of `s` where its lifetimes are.
pub fn project(impls: &[ImplDef], s: &Type, tr: &str, name: &str) -> Option<Type> {
    if let Type::Param(_) | Type::Var(_) | Type::Proj(..) = s {
        return None;
    }
    for d in impls.iter().filter(|d| d.trait_name.as_deref() == Some(tr)) {
        let mut found = HashMap::new();
        if !matches(&d.self_ty, s, &d.params, &mut found) {
            continue;
        }
        let mut regions = HashMap::new();
        regions_at(&d.self_ty, s, &d.lifetimes, &mut regions);
        let (_, rhs) = d.assoc.iter().find(|(n, _)| n == name)?;
        let args: Vec<Type> = d
            .params
            .iter()
            .map(|p| found.get(p).cloned().unwrap_or(Type::Unit))
            .collect();
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
        (Type::Box(a), Type::Box(b)) => regions_at(a, b, lifetimes, out),
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
/// - Every trait impl is *parametric* in lifetimes (`check::traits`'s
///   `parametric`): a lifetime or type parameter appears once in its type,
///   never `'static`, with no lifetime bounds. So the impl found for
///   `Pair<'x, 'y>` was checked for every two lifetimes, and what it gives
///   can't depend on which. An associated type is a type it gives:
///   `<Pair<'x, 'y> as Tr>::Out` is worked out with `'x` and `'y`, each
///   where the header has it (`project`), never with regions erased, and
///   only from what's outside function types.
/// - Nor does it look at what a function type is generic over: an impl for
///   `fn(&int) -> &int`, for every lifetime, is found for a `fn(&'a int) ->
///   &'a int`. That's sound only because every function value works at
///   every lifetime, as a closure captures no references, and because an
///   associated type can't mention a parameter that's only inside a
///   function type. Closures that capture references would break it.
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
