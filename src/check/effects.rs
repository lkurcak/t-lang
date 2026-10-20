//! Effects: what a call may do besides give its result (see
//! docs/07-effects.md). Each function says its effects in its signature,
//! `with panic`, and its body may only have those: each operator, call and
//! allocation in it is a *site* with effects of its own, which must be
//! among them. A closure's effects are worked out from its body instead,
//! as a variable that must hold at least those of the sites in it.
//!
//! Effect sets are types of a kind of their own (`Type::Effects`), so a
//! variable can stand for one, as for any type. What can't wait for
//! unification are the `⊆` that a closure's body, or an `Fn` bound, says
//! of them: each is a `Within`, kept until the end of the function, when
//! every effect variable left gets the least set that makes them all hold
//! (`settle_effects`). Sets of effects with union are a join-semilattice,
//! so that least set exists, and taking it is what makes a closure's type
//! say no more than its body does.
//!
//! Soundness (the lemma docs/07-effects.md proves): a function whose
//! effects don't have `panic` never reaches a panic. Every site's effects
//! are among its function's, or its closure's; a call's are what the
//! callee's type says, at the arguments it's called with; and what's
//! called through a type parameter, a function value or a trait's method
//! has no more effects than its bound, its type or the trait's method
//! says (`Within`, `traits::fits`, `Checker::impl_effects`).

use super::Checker;
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Lint, Span, Warning};
use crate::types::{Type, effects_text};
use std::collections::HashMap;

/// Something in a body with effects: an operator, a call or an
/// allocation, at `span`, described as `what` ("`+`", "`half`").
#[derive(Debug, Clone)]
pub(super) struct Site {
    pub span: Span,
    pub what: String,
    pub effects: Type,
}

/// `sub ⊆ sup`, of two effect sets, which must hold once the function's
/// variables are solved, because of `why`, at `span`.
#[derive(Debug, Clone)]
pub(super) struct Within {
    pub sub: Type,
    pub sup: Type,
    pub span: Span,
    pub why: Why,
}

#[derive(Debug, Clone)]
pub(super) enum Why {
    /// A site in a closure's body, whose effects must be the closure's.
    Closure(String),
    /// What's called meeting an `Fn` bound of `who`'s, which says calls
    /// are `sig`.
    Bound { who: String, sig: Type },
    /// Two sets that must be equal, as parts of types that must be.
    Same,
    /// A function of type `got` used as a `want`, which may have more.
    Coerce { got: Type, want: Type },
}

impl Checker {
    /// Records that what's at `span`, described as `what`, has `effects`,
    /// in the body being checked.
    pub(super) fn effect(&mut self, span: Span, what: impl Into<String>, effects: Type) {
        if effects.is_no_effects() {
            return;
        }
        self.effect_sites.push(Site {
            span,
            what: what.into(),
            effects,
        });
    }

    /// Records that `sub ⊆ sup` must hold, because of `why`, at `span`.
    pub(super) fn within(&mut self, sub: Type, sup: Type, span: Span, why: Why) {
        self.withins.push(Within {
            sub,
            sup,
            span,
            why,
        });
    }

    /// A new variable for the effects of the closure at `span`.
    pub(super) fn closure_effects(&mut self, span: Span) -> Type {
        let what = "the effects of this closure".to_string();
        let t = self.fresh_effects(span, what);
        if let [Type::Var(n)] = t.effect_items() {
            self.effect_owners.insert(*n, span);
        }
        t
    }

    /// A set of effects not known yet, `what` at `span`: a variable, which
    /// gets the least set that it can be at the end of the function, if
    /// nothing says what it is before.
    pub(super) fn fresh_effects(&mut self, span: Span, what: String) -> Type {
        let v = self.fresh(span, what, "say them with a bound".into());
        if let Type::Var(n) = v {
            self.effect_vars.insert(n);
        }
        Type::effects([v])
    }

    /// A fresh variable for type parameter `p`, or a set of effects not
    /// known yet if it's an effect parameter.
    pub(super) fn fresh_param(&mut self, p: &str, span: Span, what: String, hint: String) -> Type {
        match crate::parser::is_effect_name(p) {
            true => self.fresh_effects(span, what),
            false => self.fresh(span, what, hint),
        }
    }

    /// The effects of the closure whose body's sites are `sites` are
    /// `effects`: at least those of each.
    pub(super) fn closure_sites(&mut self, sites: Vec<Site>, effects: &Type) {
        for s in sites {
            let why = Why::Closure(s.what);
            self.within(s.effects, effects.clone(), s.span, why);
        }
    }

    /// Gives each effect variable not solved yet the least set that makes
    /// every `Within` hold, and then checks that they all do.
    ///
    /// The least solution: each variable is the union, over the `Within`s
    /// whose `sup` has it, of what their `sub` is, less what else `sup`
    /// has, until nothing changes. That ends, as each step only adds
    /// effects that are already somewhere in the function, of which there
    /// are finitely many. A `sup` with two variables gives the first what
    /// it needs, which is a solution if not the only least one.
    pub(super) fn settle_effects(&mut self, vars: usize) -> Result<(), Error> {
        let withins = std::mem::take(&mut self.withins);
        let mut least: HashMap<u32, Vec<Type>> = HashMap::new();
        let open = |c: &Checker, t: &Type| -> Vec<u32> {
            c.solved(t)
                .effect_items()
                .iter()
                .filter_map(|e| match e {
                    Type::Var(v) => Some(*v),
                    _ => None,
                })
                .collect()
        };
        loop {
            let mut changed = false;
            for w in &withins {
                let Some(&v) = open(self, &w.sup).first() else {
                    continue;
                };
                let sup = self.solved(&w.sup);
                let mut sub = Vec::new();
                for e in self.solved(&w.sub).effect_items() {
                    match e {
                        Type::Var(u) => sub.extend(least.get(u).into_iter().flatten().cloned()),
                        e => sub.push(e.clone()),
                    }
                }
                let have = least.entry(v).or_default();
                for e in sub {
                    if !sup.effect_items().contains(&e) && !have.contains(&e) {
                        have.push(e);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // Every effect variable of the function left gets its least set,
        // those nothing said anything of, none.
        let mut all: Vec<u32> = self
            .effect_vars
            .iter()
            .copied()
            .filter(|&v| v as usize >= vars)
            .collect();
        all.sort();
        for v in all {
            if self.vars[v as usize].solution.is_none() {
                let items = least.remove(&v).unwrap_or_default();
                self.vars[v as usize].solution = Some(Type::effects(items));
            }
        }
        for w in withins {
            let (sub, sup) = (self.solved(&w.sub), self.solved(&w.sup));
            if crate::traits::subset(&sub, &sup) {
                continue;
            }
            let missing = Type::effects(
                sub.effect_items()
                    .iter()
                    .filter(|e| !sup.effect_items().contains(e))
                    .cloned(),
            );
            let does = does(&missing);
            let subject = match self.owner(&w.sub) {
                Some(_) => "this closure".to_string(),
                None => "this function".to_string(),
            };
            let message = match &w.why {
                Why::Closure(what) => format!("{what} {does}, but this closure can't"),
                Why::Bound { who, sig } => {
                    let sig = self.solved(sig);
                    let cant = cant(&missing);
                    format!(
                        "{subject} {does}, but `{who}` needs calls of it to fit `{sig}`, which {cant}"
                    )
                }
                Why::Coerce { got, want } => format!(
                    "expected `{}`, found `{}`",
                    self.solved(want),
                    self.solved(got)
                ),
                Why::Same => format!(
                    "expected the effects `{}`, found `{}`",
                    effects_text(&sup),
                    effects_text(&sub)
                ),
            };
            let span = match (&w.why, self.owner(&w.sub)) {
                (Why::Bound { .. }, Some(at)) => at,
                _ => w.span,
            };
            return Err(Error::ty(span, message));
        }
        Ok(())
    }

    /// Where the closure is whose effects `t` is, if it's a closure's.
    fn owner(&self, t: &Type) -> Option<Span> {
        match t.effect_items() {
            [Type::Var(v)] => self.effect_owners.get(v).copied(),
            _ => None,
        }
    }

    /// Checks the sites of the body of `f`, whose effects are `declared`,
    /// against them, and warns of each of them that no site has.
    pub(super) fn body_effects(&mut self, f: &FnDecl, declared: &Type) -> Result<(), Error> {
        let sites = std::mem::take(&mut self.effect_sites);
        let mut used: Vec<Type> = Vec::new();
        for s in &sites {
            let effects = self.solved(&s.effects);
            let missing: Vec<Type> = effects
                .effect_items()
                .iter()
                .filter(|e| !declared.effect_items().contains(e))
                .cloned()
                .collect();
            used.extend(effects.effect_items().iter().cloned());
            let Some(first) = missing.first() else {
                continue;
            };
            let all = Type::effects(declared.effect_items().iter().chain(&missing).cloned());
            let message = format!(
                "{} {}, but `{}` doesn't say `with {}`",
                s.what,
                does(&Type::effects([first.clone()])),
                f.name.name,
                effects_text(&Type::effects([first.clone()])),
            );
            let fix = Fix::new(
                format!("say `with {}`", effects_text(&all)),
                vec![Edit {
                    span: effects_span(f),
                    text: format!(" with {}", written_with(f, &all)),
                }],
            );
            return Err(Error::ty(s.span, message).with_fix(fix));
        }
        // Each effect it says that nothing in it has. One that's an
        // associated effect with none, as `Self::e` may be, says nothing.
        // Only where nothing outside can count on it: not a `pub`
        // function, which may say an effect to keep the right to have it
        // later, nor a trait's method, default or impl, which is public
        // wherever the trait is, and says what its impls may do.
        let Some(written) = &f.effects else {
            return Ok(());
        };
        let default = self.self_ty.is_none() && self.type_params.iter().any(|p| p == "Self");
        let of_trait = default || self.in_trait_impl;
        if f.public.is_some() || of_trait {
            return Ok(());
        }
        let resolved: Vec<Type> = written
            .items
            .iter()
            .map(|i| self.written_effect(i))
            .collect();
        let unused: Vec<bool> = resolved
            .iter()
            .map(|r| !r.is_no_effects() && !r.effect_items().iter().any(|e| used.contains(e)))
            .collect();
        for (k, item) in written.items.iter().enumerate() {
            if !unused[k] {
                continue;
            }
            let keep: Vec<String> = (written.items.iter().enumerate())
                .filter(|&(j, _)| j != k)
                .map(|(_, i)| i.to_string())
                .collect();
            let text = match keep.len() {
                0 => String::new(),
                1 => format!(" with {}", keep[0]),
                _ => format!(" with {{{}}}", keep.join(", ")),
            };
            let message = match resolved[k].effect_items() {
                [Type::Panic] => format!("`{}` says `with panic`, but it can't panic", f.name.name),
                [Type::Alloc] => {
                    format!("`{}` says `with alloc`, but it can't allocate", f.name.name)
                }
                _ => format!(
                    "`{}` says `with {item}`, but nothing in it has the effects `{item}`",
                    f.name.name
                ),
            };
            let fix = Fix::new(
                match keep.is_empty() {
                    true => "remove the `with`".to_string(),
                    false => format!("remove `{item}`"),
                },
                vec![Edit {
                    span: effects_span(f),
                    text,
                }],
            );
            self.warnings.push(Warning {
                lint: Lint::UnusedEffects,
                span: item.span,
                message,
                fixes: vec![fix],
            });
        }
        Ok(())
    }

    /// The effects `item`, written in the signature of the function being
    /// checked, are, as its type has them.
    fn written_effect(&self, item: &Effect) -> Type {
        let set = EffectSet {
            items: vec![item.clone()],
            span: item.span,
            braced: false,
            with: true,
        };
        let mut lts = super::Lifetimes::Any;
        let Ok(t) = self.resolve_effects_with(&set, &self.type_params, &mut lts) else {
            return Type::no_effects();
        };
        let t = match &self.self_ty {
            Some(s) => {
                crate::types::Subst::new(&["Self".to_string()], std::slice::from_ref(s)).apply(&t)
            }
            None => t,
        };
        self.solved(&t)
    }
}

/// What effects `missing` mean a call may do: "may panic".
pub(super) fn does(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "may panic".into(),
        [Type::Alloc] => "may allocate".into(),
        [Type::Alloc, Type::Panic] => "may allocate and panic".into(),
        _ => format!("may have the effects `{}`", effects_text(missing)),
    }
}

/// What a function type without effects `missing` can't do.
fn cant(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "can't panic".into(),
        [Type::Alloc] => "can't allocate".into(),
        _ => format!("can't have the effects `{}`", effects_text(missing)),
    }
}

/// From the end of `f`'s result, or its parameters if it has none
/// written, to the end of its effects, if it says any: where `with ..`
/// goes.
fn effects_span(f: &FnDecl) -> Span {
    let start = f.ret.span.end;
    let end = f.effects.as_ref().map_or(start, |e| e.span.end);
    Span::new(start, end)
}

/// Where `f`'s effects are, or would be, written, and each that's written.
pub(crate) fn written_at(f: &FnDecl) -> (Span, Vec<String>) {
    let items = f.effects.iter().flat_map(|e| &e.items);
    (effects_span(f), items.map(Effect::to_string).collect())
}

/// `panic`, or `{alloc, panic}`, of effects as written.
pub(super) fn set_text(items: &[String]) -> String {
    match items {
        [one] => one.clone(),
        _ => format!("{{{}}}", items.join(", ")),
    }
}

/// `all`, as `f` would say it: with the effects it says as written, and
/// then the others.
fn written_with(f: &FnDecl, all: &Type) -> String {
    let mut items: Vec<String> = f
        .effects
        .iter()
        .flat_map(|e| &e.items)
        .map(Effect::to_string)
        .collect();
    for e in all.effect_items() {
        let text = e.to_string();
        if !items.contains(&text) {
            items.push(text);
        }
    }
    match items.len() {
        1 => items.remove(0),
        _ => format!("{{{}}}", items.join(", ")),
    }
}
