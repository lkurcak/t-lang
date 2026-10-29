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
//! An `unsafe` block takes the effect `unsafe` away from the sites in it,
//! and nothing else (`Checker::unsafe_block`). An effect parameter `e`
//! stays `e`, whatever a caller makes it, as the block can't know those
//! obligations; and a closure's sites are its body's, not those of the
//! block it's made in.
//!
//! Soundness (the lemma docs/07-effects.md proves): a function whose
//! effects don't have `panic` never reaches a panic, nor one whose effects
//! don't have `unsafe` an unsafe operation outside an `unsafe` block
//! (docs/09-unsafe.md). Every site's effects are among its function's, or
//! its closure's, but the `unsafe` a block takes away; a call's are what
//! the callee's type says, at the arguments it's called with; and what's
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
    /// The operator it is, if it's one: proving it can't panic takes it
    /// out (see `refine`).
    pub node: Option<NodeId>,
    /// The innermost `unsafe` block it's in, if it's in one, in its body:
    /// that keeps its obligations, so it doesn't have the effect `unsafe`.
    pub unsafe_block: Option<Span>,
}

/// `sub ⊆ sup`, of two effect sets, which must hold once the function's
/// variables are solved, because of `why`, at `span`.
#[derive(Debug, Clone)]
pub(super) struct Within {
    pub sub: Type,
    pub sup: Type,
    pub span: Span,
    pub why: Why,
    /// The operator it's for, if it's a site of one in a closure.
    pub node: Option<NodeId>,
    /// Is it for a site in an `unsafe` block, whose `unsafe` doesn't
    /// count?
    pub discharged: bool,
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
            node: None,
            unsafe_block: None,
        });
    }

    /// Records that operator `e`, described as `what`, may panic, unless
    /// it's proved not to (see `refine`).
    pub(super) fn operator(&mut self, e: &Expr, what: impl Into<String>) {
        self.effect_sites.push(Site {
            span: e.span,
            what: what.into(),
            effects: Type::effects([Type::Panic]),
            node: Some(e.id),
            unsafe_block: None,
        });
    }

    /// Records that `sub ⊆ sup` must hold, because of `why`, at `span`.
    pub(super) fn within(&mut self, sub: Type, sup: Type, span: Span, why: Why) {
        self.withins.push(Within {
            sub,
            sup,
            span,
            why,
            node: None,
            discharged: false,
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

    /// A fresh variable for parameter `p` of `params`, or a set of effects
    /// not known yet if it's an effect parameter.
    pub(super) fn fresh_param(
        &mut self,
        params: &crate::types::Params,
        p: &str,
        span: Span,
        what: String,
        hint: String,
    ) -> Type {
        match params.is_effect(p) {
            true => self.fresh_effects(span, what),
            false => {
                let t = self.fresh(span, what, hint);
                if let Type::Var(v) = t {
                    self.vars[v as usize].param = true;
                }
                t
            }
        }
    }

    /// The effects of the closure whose body's sites are `sites` are
    /// `effects`: at least those of each.
    pub(super) fn closure_sites(&mut self, sites: Vec<Site>, effects: &Type) {
        for s in sites {
            let why = Why::Closure(s.what);
            self.within(s.effects, effects.clone(), s.span, why);
            let w = self.withins.last_mut().expect("pushed");
            w.node = s.node;
            w.discharged = s.unsafe_block.is_some();
        }
    }

    /// Checks `unsafe` block `e`, of `block`, wanted as `want` if that's
    /// known. Each site in it that's not in an `unsafe` block inside it is
    /// in this one, which keeps its obligations: its effect `unsafe` is
    /// taken away. Only that effect, as the operations in the block have
    /// it, not what an effect parameter turns out to be: an `unsafe { f() }`
    /// of an `f: F` with effects `e` still has `e`, as the block can't
    /// know what obligations `f` has. Nor what a closure made in it does
    /// when it's called, later: a closure's sites are its body's.
    pub(super) fn unsafe_block(
        &mut self,
        e: &Expr,
        block: &Expr,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let ExprKind::Block(stmts, tail) = &block.kind else {
            unreachable!("`unsafe` is parsed with a block")
        };
        let start = self.effect_sites.len();
        let t = self.block(stmts, tail.as_deref(), want)?;
        self.typed.exprs.insert(block.id, t.clone());
        for s in &mut self.effect_sites[start..] {
            if s.unsafe_block.is_none() {
                s.unsafe_block = Some(e.span);
                self.unsafe_uses.push((e.span, s.effects.clone()));
            }
        }
        self.unsafe_blocks.push(e.span);
        Ok(t)
    }

    /// Warns of each `unsafe` block of the body just checked with no site
    /// in it that's unsafe: it keeps no obligations, and only hides where
    /// the unsafe operations aren't.
    fn unused_unsafe(&mut self) {
        let uses = std::mem::take(&mut self.unsafe_uses);
        for block in std::mem::take(&mut self.unsafe_blocks) {
            let used = uses.iter().any(|(b, effects)| {
                *b == block && self.solved(effects).effect_items().contains(&Type::Unsafe)
            });
            if used {
                continue;
            }
            let keyword = Span::new(block.start, block.start + "unsafe ".len());
            self.warnings.push(Warning {
                lint: Lint::UnusedUnsafe,
                span: Span::new(block.start, block.start + "unsafe".len()),
                message: "this `unsafe` block has nothing unsafe in it".into(),
                fixes: vec![Fix::new(
                    "remove the `unsafe`",
                    vec![Edit {
                        span: keyword,
                        text: String::new(),
                    }],
                )],
            });
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
                if w.discharged {
                    sub.retain(|e| *e != Type::Unsafe);
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
            let (mut sub, sup) = (self.solved(&w.sub), self.solved(&w.sup));
            if w.discharged {
                sub = without_unsafe(&sub);
            }
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
        self.unused_unsafe();
        let sites = std::mem::take(&mut self.effect_sites);
        let mut used: Vec<Type> = Vec::new();
        for s in &sites {
            let mut effects = self.solved(&s.effects);
            if s.unsafe_block.is_some() {
                effects = without_unsafe(&effects);
            }
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
            // Nothing calls `main` to keep its obligations, so it can't
            // say `with unsafe` (see `check_main`).
            let main = self
                .current
                .is_some_and(|id| self.fns[id.0].symbol == "main");
            if *first == Type::Unsafe && main {
                let message = format!("{} is unsafe, but it isn't in an `unsafe` block", s.what);
                return Err(Error::ty(s.span, message));
            }
            let message = match first {
                Type::Unsafe => format!(
                    "{} is unsafe, but it isn't in an `unsafe` block, and `{}` doesn't say \
                     `with unsafe`",
                    s.what, f.name.name,
                ),
                _ => format!(
                    "{} {}, but `{}` doesn't say `with {}`",
                    s.what,
                    does(&Type::effects([first.clone()])),
                    f.name.name,
                    effects_text(&Type::effects([first.clone()])),
                ),
            };
            // An impl's method can say no more than its trait's (see
            // `impls`), so saying it there isn't a fix.
            if self.in_trait_impl {
                let note = format!(
                    "`{}` is a method of a trait's impl, which may do only what the trait's \
                     method says it may",
                    f.name.name
                );
                return Err(Error::ty(s.span, message).with_note(f.name.span, note));
            }
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
        // Only where nothing outside the package can count on it: not an
        // exported function, which may say an effect to keep the right to
        // have it later, nor a trait's method, default or impl, which is
        // seen wherever the trait is, and says what its impls may do.
        // Code in the package that counts on it is checked with it.
        let Some(written) = &f.effects else {
            return Ok(());
        };
        let default = self.self_ty.is_none() && self.type_params.iter().any(|p| p == "Self");
        let of_trait = default || self.in_trait_impl;
        if f.vis.is_export() || of_trait {
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
                [Type::Unsafe] => format!(
                    "`{}` says `with unsafe`, but nothing in it is unsafe outside an `unsafe` \
                     block",
                    f.name.name
                ),
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

/// `effects` but `unsafe`, which an `unsafe` block takes away.
fn without_unsafe(effects: &Type) -> Type {
    let items = effects.effect_items().iter();
    Type::effects(items.filter(|e| **e != Type::Unsafe).cloned())
}

/// What effects `missing` mean a call may do: "may panic".
pub(super) fn does(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "may panic".into(),
        [Type::Alloc] => "may allocate".into(),
        [Type::Alloc, Type::Panic] => "may allocate and panic".into(),
        [Type::Unsafe] => "is unsafe".into(),
        _ => format!("may have the effects `{}`", effects_text(missing)),
    }
}

/// What a function type without effects `missing` can't do.
fn cant(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "can't panic".into(),
        [Type::Alloc] => "can't allocate".into(),
        [Type::Unsafe] => "can't be unsafe".into(),
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
