//! Effects: what a call may do besides give its result (see
//! docs/07-effects.md). A function says its effects in its signature,
//! `with panic`, and its body may only have those: each operator, call and
//! allocation in it is a *site* with effects of its own, which must be
//! among them. One that isn't exported and says no `with` has its sites'
//! instead, inferred with those it may call back (see `check/infer.rs`):
//! until they're known its effects are a placeholder, and what a
//! placeholder is in waits for them (`Deferred`). A closure's effects are worked out from its body instead,
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
//!
//! Since step 90 an effect a function says may have a condition, `with
//! panic if P` (docs/90-conditional-effects.md). Invariant: an effect is
//! taken out of a site only by a proof, on every path the walk takes to
//! it, that it can't happen there (`Walk::cut_effects`: the callee's
//! condition is ruled out), and a site may have one of its function's
//! conditional effects only where a proof says the condition held at the
//! call (`Site::covered`); every other site is checked against the
//! effects the function has always.

use super::Checker;
use super::infer::has_placeholder;
use super::scope::FnId;
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Lint, Span, Warning};
use crate::traits::Pred;
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
    /// The function it calls, by symbol, if it's a call of one by name: to
    /// say why it has an effect, if it's inferred (`why_inferred`).
    pub callee: Option<String>,
    /// The call it is, or `Box::new`, by node, if it's one: the walk may
    /// find some of its effects can't happen there, which are taken out
    /// (docs/90-conditional-effects.md).
    pub at: Option<NodeId>,
    /// Its callee's type arguments, if it's a call of a function by name
    /// that says an effect with `unless`: whether its bound holds of them
    /// decides if the call has it (`Checker::unguarded`).
    pub targs: Vec<Type>,
    /// The function's own effects it may have only where their conditions
    /// hold, as the walk found: which it doesn't need its function to have
    /// always, though they're of use (lemma C2 of
    /// docs/90-conditional-effects.md).
    pub covered: Vec<Type>,
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
    /// The call it's for, if it's a site of one in a closure (`Site::at`).
    pub at: Option<NodeId>,
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
    /// A site `what` in the body of function `name`, which says its
    /// effects, which the site's must be among.
    Site { what: String, name: String },
    /// An associated effect, `what` where it's used, which a bound says
    /// is within a set, as `needs` says (docs/68.1-effect-bindings.md).
    Assoc { what: Type, needs: String },
}

/// A `Within` that had a placeholder for a function's inferred effects in
/// it at the end of the function it's from, kept until they're known
/// (see `check/infer.rs`): what it says, with that function's variables
/// solved, and what to say if it doesn't hold, of what and where.
#[derive(Debug, Clone)]
pub(super) struct Deferred {
    pub sub: Type,
    pub sup: Type,
    pub why: Why,
    pub subject: String,
    pub span: Span,
    /// The bounds on associated effects where it was found
    /// (`Checker::effect_bounds`).
    pub bounds: Vec<(Type, Type)>,
}

impl Checker {
    /// Records that what's at `span`, a call or a static's use, of node
    /// `at` if it's one the walk sees, described as `what`, of function
    /// `callee` if it's a call of one named, has `effects`, in the body
    /// being checked.
    pub(super) fn call_effect(
        &mut self,
        span: Span,
        at: Option<NodeId>,
        what: impl Into<String>,
        effects: Type,
        callee: Option<String>,
    ) {
        if effects.is_no_effects() {
            return;
        }
        self.effect_sites.push(Site {
            span,
            what: what.into(),
            effects,
            node: None,
            unsafe_block: None,
            callee,
            at,
            targs: Vec::new(),
            covered: Vec::new(),
        });
    }

    /// Says what the type arguments of the call at node `at` are, of a
    /// callee that says an effect with `unless`.
    pub(super) fn call_targs(&mut self, at: NodeId, callee: &str, targs: &[Type]) {
        if !self.unless.contains_key(callee) {
            return;
        }
        if let Some(s) = self
            .effect_sites
            .iter_mut()
            .rev()
            .find(|s| s.at == Some(at))
        {
            s.targs = targs.to_vec();
        }
    }

    /// `effects`, those of site `s`, without what its callee says it has
    /// `unless` a bound that holds of the call's type arguments, where the
    /// body is being checked (lemma B5 of docs/92-where-a-bound-holds.md).
    /// Only what the `unless` put there: an effect parameter or an
    /// associated effect of the callee's that's the same effect at the
    /// call keeps it.
    pub(super) fn unguarded(&self, s: &Site, effects: Type) -> Type {
        let Some(callee) = &s.callee else {
            return effects;
        };
        let (Some(guards), Some(scheme)) = (self.unless.get(callee), self.callee_scheme(callee))
        else {
            return effects;
        };
        let Type::Fn(sig) = &scheme.ty else {
            return effects;
        };
        let names = scheme.params.as_slice();
        if names.len() != s.targs.len() {
            return effects;
        }
        let subst = crate::types::Subst::new(names, &s.targs);
        let mut gone = Vec::new();
        for g in guards {
            let ty = self.solved(&subst.apply(&g.pred.ty));
            if crate::check::unify::has_vars(&ty) {
                continue;
            }
            let pred = Pred {
                ty,
                trait_name: g.pred.trait_name.clone(),
            };
            if !self.pred_holds(&pred) {
                continue;
            }
            let rest = (sig.effects.effect_items().iter())
                .filter(|e| **e != g.atom)
                .map(|e| self.adts.normalise(&self.solved(&subst.apply(e))));
            if Type::effects(rest).effect_items().contains(&g.atom) {
                continue;
            }
            gone.push(g.atom.clone());
        }
        Type::effects(
            (effects.effect_items().iter())
                .filter(|e| !gone.contains(e))
                .cloned(),
        )
    }

    /// The scheme of function `symbol`, a function's or a trait's method's.
    pub(super) fn callee_scheme(&self, symbol: &str) -> Option<&crate::check::Scheme> {
        if let Some(f) = self.fn_def(symbol) {
            return Some(&f.scheme);
        }
        (self.traits.traits.values().flat_map(|t| &t.methods))
            .find(|m| m.symbol == symbol)
            .map(|m| &m.scheme)
    }

    /// The effects the function being checked, `symbol`, may not have
    /// though its signature has them, each with why: one it says `unless`
    /// a bound that holds here; and, of an impl's method of a trait, one
    /// its trait's method says `unless` a bound that the impl's type may
    /// meet, as a call through the trait there has none (lemma B6 of
    /// docs/92-where-a-bound-holds.md).
    pub(super) fn barred(&self, symbol: &str) -> Vec<(Type, String)> {
        let mut out = Vec::new();
        for g in self.unless.get(symbol).into_iter().flatten() {
            if self.pred_holds(&g.pred) {
                out.push((g.atom.clone(), g.text.clone()));
            }
        }
        let of_impl = (self.traits.impls.iter()).find_map(|d| {
            let tr = d.trait_name.as_ref()?;
            let (name, _) = d.methods.iter().find(|(_, s)| s == symbol)?;
            Some((
                crate::check::traits::trait_method_symbol(tr, name),
                d.self_ty.clone(),
            ))
        });
        if let Some((method, this)) = of_impl {
            for g in self.unless.get(&method).into_iter().flatten() {
                let pred = Pred {
                    ty: this.clone(),
                    trait_name: g.pred.trait_name.clone(),
                };
                let open = matches!(this, Type::Param(..) | Type::Proj(..) | Type::Opaque(..));
                let local = &self.adts.proj.local;
                let may = open
                    || self.pred_holds(&pred)
                    || self
                        .traits
                        .select(&this, Some(&pred.trait_name), local)
                        .is_some();
                if may {
                    let tr = crate::check::source_name(&pred.trait_name);
                    out.push((g.atom.clone(), format!("{this}: {tr}")));
                }
            }
        }
        out
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
            callee: None,
            at: None,
            targs: Vec::new(),
            covered: Vec::new(),
        });
    }

    /// Records that what each of `bounds`, a function's (or an opaque
    /// type's) bounds on associated effects, says holds at a use of it at
    /// `span` by `who`, at type arguments `s`: the associated effect there,
    /// normalised, is within the set (docs/68.1-effect-bindings.md).
    pub(super) fn effect_bounds_hold(
        &mut self,
        bounds: &[(Type, Type)],
        s: &crate::types::Subst,
        span: Span,
        who: &str,
    ) {
        for (lhs, rhs) in bounds {
            let sub = Type::effects([self.adts.normalise(&s.apply(lhs))]);
            let why = Why::Assoc {
                what: s.apply(lhs),
                needs: format!("`{who}` needs `{lhs} = {}`", effects_text(rhs)),
            };
            self.within(sub, s.apply(rhs), span, why);
        }
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
            at: None,
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
                    self.vars[v as usize].unsized_ok = params.is_unsized(p);
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
            w.at = s.at;
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
                notes: Vec::new(),
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
        let withins = self.least_effects(vars);
        self.check_withins(withins)
    }

    /// `settle_effects`' least solution, without its checks: the `Within`s,
    /// taken, for those.
    pub(super) fn least_effects(&mut self, vars: usize) -> Vec<Within> {
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
        withins
    }

    /// `settle_effects`' checks, of what's within what.
    fn check_withins(&mut self, withins: Vec<Within>) -> Result<(), Error> {
        for w in withins {
            let (mut sub, sup) = (self.solved(&w.sub), self.solved(&w.sup));
            if w.discharged {
                sub = without_unsafe(&sub);
            }
            let subject = match self.owner(&w.sub) {
                Some(_) => "this closure".to_string(),
                None => "this function".to_string(),
            };
            let span = match (&w.why, self.owner(&w.sub)) {
                (Why::Bound { .. }, Some(at)) => at,
                _ => w.span,
            };
            let why = match w.why {
                Why::Bound { who, sig } => Why::Bound {
                    who,
                    sig: self.solved(&sig),
                },
                Why::Coerce { got, want } => Why::Coerce {
                    got: self.solved(&got),
                    want: self.solved(&want),
                },
                // The projection itself, of what's known now, not what it
                // normalises to: `<Down as Iterator>::e`.
                Why::Assoc { what, needs } => Why::Assoc {
                    what: match &what {
                        Type::Proj(base, tr, e) => {
                            Type::Proj(Box::new(self.solved(base)), tr.clone(), e.clone())
                        }
                        what => self.solved(what),
                    },
                    needs,
                },
                why => why,
            };
            let d = Deferred {
                sub,
                sup,
                why,
                subject,
                span,
                bounds: self.effect_bounds.clone(),
            };
            // A function's effects that are inferred, and not known yet,
            // wait for them.
            if has_placeholder(&d.sub) || has_placeholder(&d.sup) {
                self.deferred.push(d);
                continue;
            }
            self.check_deferred(&d)?;
        }
        Ok(())
    }

    /// Checks that `d` holds, by now: that its `sub` is within its `sup`,
    /// or else says why it must be.
    pub(super) fn check_deferred(&self, d: &Deferred) -> Result<(), Error> {
        let (sub, sup) = (&d.sub, &d.sup);
        let missing = Type::effects(self.missing_effects(sub, sup, &d.bounds));
        if missing.is_no_effects() {
            return Ok(());
        }
        let does = does(&missing);
        let subject = &d.subject;
        let shown = |t: &Type| self.with_inferred(t);
        let message = match &d.why {
            Why::Closure(what) => format!("{what} {does}, but this closure can't"),
            Why::Bound { who, sig } => {
                let sig = shown(sig);
                let cant = cant(&missing);
                format!(
                    "{subject} {does}, but `{who}` needs calls of it to fit `{sig}`, which {cant}"
                )
            }
            Why::Coerce { got, want } => {
                format!("expected `{}`, found `{}`", shown(want), shown(got))
            }
            Why::Same => format!(
                "expected the effects `{}`, found `{}`",
                effects_text(sup),
                effects_text(sub)
            ),
            Why::Assoc { what, needs } => format!("`{}` {does}, but {needs}", shown(what)),
            Why::Site { what, name } => format!(
                "{}, but `{name}` doesn't say `with {}`",
                says(what, &missing),
                effects_text(&missing)
            ),
        };
        let error = Error::ty(d.span, message);
        Err(if missing.effect_items().contains(&Type::Panic) {
            if d.subject == "this closure" {
                self.explain_closure_proof(error)
            } else {
                self.explain_proof(error)
            }
        } else {
            error
        })
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
        self.hole_sites(&sites);
        let id = self.current.expect("in a function");
        if f.is_const() {
            for s in &sites {
                if let Some(at) = s.unsafe_block {
                    return Err(self.const_unsafe(f, at));
                }
                let effects = self.solved(&s.effects);
                if has_placeholder(&effects) {
                    self.deferred.push(Deferred {
                        sub: effects,
                        sup: Type::effects([Type::Panic]),
                        why: Why::Site {
                            what: s.what.clone(),
                            name: f.name.name.clone(),
                        },
                        subject: String::new(),
                        span: s.span,
                        bounds: self.effect_bounds.clone(),
                    });
                    continue;
                }
                for e in effects.effect_items() {
                    self.const_effect(f, e, &s.what, s.span)?;
                }
            }
            return Ok(());
        }
        if self.inferring.contains(&id.0) {
            return self.inferred_sites(id, sites);
        }
        // What it says it has only when a condition holds isn't what it
        // has always: a site may have one only where the walk found it
        // does, `covered` (lemma C2 of docs/90-conditional-effects.md).
        let symbol = self.fns[id.0].symbol.clone();
        let conds: Vec<(Type, String)> = (self.refined.get(&symbol))
            .map(|s| {
                (s.conds.iter())
                    .map(|c| (c.atom.clone(), c.text.clone()))
                    .collect()
            })
            .unwrap_or_default();
        // Nor what it says `unless` a bound that holds here, or that its
        // trait's method does, of an impl's type that may meet it.
        let barred = self.barred(&symbol);
        let always = Type::effects(
            (declared.effect_items().iter())
                .filter(|e| !conds.iter().any(|(a, _)| a == *e))
                .filter(|e| !barred.iter().any(|(a, _)| a == *e))
                .cloned(),
        );
        let declared = &always;
        let mut used: Vec<Type> = Vec::new();
        for s in &sites {
            let mut effects = self.solved(&s.effects);
            if s.unsafe_block.is_some() {
                effects = without_unsafe(&effects);
            }
            effects = self.unguarded(s, effects);
            used.extend(s.covered.iter().cloned());
            let here = Type::effects(declared.effect_items().iter().chain(&s.covered).cloned());
            let declared = &here;
            // What's in it of a function whose effects are being inferred
            // waits for them.
            if has_placeholder(&effects) {
                self.deferred.push(Deferred {
                    sub: effects,
                    sup: declared.clone(),
                    why: Why::Site {
                        what: s.what.clone(),
                        name: f.name.name.clone(),
                    },
                    subject: String::new(),
                    span: s.span,
                    bounds: self.effect_bounds.clone(),
                });
                continue;
            }
            let missing = self.missing_effects(&effects, declared, &self.effect_bounds);
            used.extend(effects.effect_items().iter().cloned());
            used.extend(self.within_bounds(&effects, &self.effect_bounds));
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
            // One it says it has unless a bound holds, which does here.
            if let Some((_, bound)) = barred.iter().find(|(a, _)| a == first) {
                let atom = effects_text(&Type::effects([first.clone()]));
                let message = format!(
                    "{} where `{bound}`, but `{}` says `with {atom} unless {bound}`",
                    says(&s.what, &Type::effects([first.clone()])),
                    f.name.name,
                );
                let err = Error::ty(s.span, message);
                return Err(if *first == Type::Panic {
                    self.explain_proof(err)
                } else {
                    err
                });
            }
            // One it says it has when a condition holds, which may not here.
            if let Some((_, cond)) = conds.iter().find(|(a, _)| a == first) {
                let atom = effects_text(&Type::effects([first.clone()]));
                // A static's use says what it is: here, that it's used.
                let does = match s.what.strip_suffix(" is a static") {
                    Some(name) => format!("{name} is used"),
                    None => says(&s.what, &Type::effects([first.clone()])),
                };
                let message = format!(
                    "{does} when `{cond}` doesn't hold, but `{}` says `with {atom} if {cond}`",
                    f.name.name,
                );
                let err = Error::ty(s.span, message);
                return Err(if *first == Type::Panic {
                    self.explain_proof(err)
                } else {
                    err
                });
            }
            let message = match first {
                Type::Unsafe => format!(
                    "{} is unsafe, but it isn't in an `unsafe` block, and `{}` doesn't say \
                     `with unsafe`",
                    s.what, f.name.name,
                ),
                _ => format!(
                    "{}, but `{}` doesn't say `with {}`",
                    says(&s.what, &Type::effects([first.clone()])),
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
                let error = Error::ty(s.span, message).with_note(f.name.span, note);
                return Err(if *first == Type::Panic {
                    self.explain_proof(error)
                } else {
                    error
                });
            }
            // Its conditions kept, if it has any.
            let label = match conds.is_empty() {
                true => effects_text(&all),
                false => written_with(f, &all),
            };
            let fix = Fix::new(
                format!("say `with {label}`"),
                vec![Edit {
                    span: effects_span(f),
                    text: format!(" with {}", written_with(f, &all)),
                }],
            );
            let mut err = Error::ty(s.span, message).with_fix(fix);
            // Why what it calls has it, if that's inferred, as it's
            // nowhere written.
            if let Some(callee) = &s.callee {
                for (at, note) in self.why_inferred(callee, first) {
                    err = err.with_note(at, note);
                }
            }
            return Err(if *first == Type::Panic {
                self.explain_proof(err)
            } else {
                err
            });
        }
        // Under the bound of each effect it says with `unless`, nothing in
        // it has the effect: what the walk under the bound left, of what
        // the walk without it did (lemma B4 of
        // docs/92-where-a-bound-holds.md). A closure's effects are what
        // they are without the bound, and a function's whose effects are
        // still being inferred aren't known: either has it, for this.
        for sc in std::mem::take(&mut self.scenarios) {
            self.adts.proj.env.push(sc.guard.pred.clone());
            let mut left = None;
            for s in &sites {
                if s.node.is_some_and(|n| sc.proved.contains(&n)) {
                    continue;
                }
                let mut effects = self.solved(&s.effects);
                if let Some(gone) = s.at.or(s.node).and_then(|n| sc.excused.get(&n)) {
                    effects = Type::effects(
                        (effects.effect_items().iter())
                            .filter(|e| !gone.contains(e))
                            .cloned(),
                    );
                }
                let effects = self.unguarded(s, effects);
                if effects.effect_items().contains(&sc.guard.atom) || has_placeholder(&effects) {
                    left = Some(s);
                    break;
                }
            }
            self.adts.proj.env.pop();
            if let Some(s) = left {
                let atom = Type::effects([sc.guard.atom.clone()]);
                let message = format!(
                    "{} where `{}`, but `{}` says `with {} unless {}`",
                    says(&s.what, &atom),
                    sc.guard.text,
                    f.name.name,
                    effects_text(&atom),
                    sc.guard.text,
                );
                let err = Error::ty(s.span, message);
                return Err(if sc.guard.atom == Type::Panic {
                    self.explain_proof(err)
                } else {
                    err
                });
            }
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
        // Nor once a proof has failed, which is assumed after: what's left
        // to panic, say, may be what follows from it.
        if !self.proof_errors.is_empty() {
            return Ok(());
        }
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
                notes: Vec::new(),
                lint: Lint::UnusedEffects,
                span: item.span,
                message,
                fixes: vec![fix],
            });
        }
        Ok(())
    }

    /// Keeps those of `sites`, of a body, that are in holes of format
    /// strings, with their effects as far as they're known.
    fn hole_sites(&mut self, sites: &[Site]) {
        let holes = &self.modules.holes;
        let in_hole =
            |s: &Site| (holes.iter()).any(|h| h.start <= s.span.start && s.span.end <= h.end);
        for s in sites.iter().filter(|s| in_hole(s)) {
            let mut s = s.clone();
            s.effects = self.solved(&s.effects);
            if s.unsafe_block.is_some() {
                s.effects = without_unsafe(&s.effects);
            }
            self.hole_sites.push(s);
        }
    }

    /// Warns of each site in a hole of a format string that has effects
    /// (`format_hole_effects`, off unless asked for): once every
    /// function's effects are known, as a call's may be inferred.
    pub(super) fn hole_effects(&mut self) {
        for s in std::mem::take(&mut self.hole_sites) {
            let effects = self.with_inferred(&s.effects);
            if effects.is_no_effects() {
                continue;
            }
            self.warnings.push(Warning {
                notes: Vec::new(),
                lint: Lint::FormatHoleEffects,
                span: s.span,
                message: format!(
                    "{} in a hole of a format string; work it out in a `let` first",
                    says(&s.what, &effects)
                ),
                fixes: Vec::new(),
            });
        }
    }

    /// Keeps the sites of the body of the function `id`, whose effects are
    /// inferred, for when they're worked out with those of its group
    /// (`solve_group`), with its variables solved.
    fn inferred_sites(&mut self, id: FnId, sites: Vec<Site>) -> Result<(), Error> {
        let mut kept = Vec::new();
        for mut s in sites {
            s.effects = self.solved(&s.effects);
            if s.unsafe_block.is_some() {
                s.effects = without_unsafe(&s.effects);
            }
            s.effects = self.unguarded(&s, s.effects.clone());
            kept.push(s);
        }
        self.infer_sites.insert(id.0, kept);
        Ok(())
    }

    /// The effects `item`, written in the signature of the function being
    /// checked, are, as its type has them.
    fn written_effect(&self, item: &Effect) -> Type {
        let set = EffectSet {
            items: vec![item.clone()],
            span: item.span,
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

impl Checker {
    /// What an associated effect `item` is at most, if a bound on it says
    /// (docs/68.1-effect-bindings.md): one of `bounds`, those in scope
    /// where it's used, or of the opaque type it's of. `I::e` stays itself
    /// in types; only where effects must be within others is this used,
    /// as it's an upper bound.
    pub(super) fn effect_bound(&self, item: &Type, bounds: &[(Type, Type)]) -> Option<Type> {
        let Type::Proj(base, tr, name) = item else {
            return None;
        };
        if let Some((_, r)) = bounds.iter().find(|(l, _)| self.solved(l) == *item) {
            return Some(self.solved(r));
        }
        let Type::Opaque(s, _, args) = &**base else {
            return None;
        };
        let def = self.traits.opaques.get(s)?;
        let (_, r) = (def.effect_bounds.iter())
            .find(|(l, _)| matches!(l, Type::Proj(_, t, n) if t == tr && n == name))?;
        let names: Vec<String> = def.params.iter().cloned().collect();
        if names.len() != args.len() {
            return None;
        }
        Some(self.solved(&crate::types::Subst::new(&names, args).apply(r)))
    }

    /// The effects of `sub` that `sup` doesn't have, with each associated
    /// effect a bound says is within a set, `bounds` or its opaque type's,
    /// replaced by what of that set `sup` doesn't have.
    pub(super) fn missing_effects(
        &self,
        sub: &Type,
        sup: &Type,
        bounds: &[(Type, Type)],
    ) -> Vec<Type> {
        fn go(
            c: &Checker,
            sub: &Type,
            sup: &Type,
            bounds: &[(Type, Type)],
            depth: usize,
        ) -> Vec<Type> {
            let mut out = Vec::new();
            for e in sub.effect_items() {
                if sup.effect_items().contains(e) {
                    continue;
                }
                match c.effect_bound(e, bounds) {
                    Some(b) if depth < 8 => out.extend(go(c, &b, sup, bounds, depth + 1)),
                    _ => out.push(e.clone()),
                }
            }
            out
        }
        go(self, sub, sup, bounds, 0)
    }

    /// What the associated effects in `effects` are at most, by `bounds`
    /// or their opaque types': used, where a `with` says them.
    fn within_bounds(&self, effects: &Type, bounds: &[(Type, Type)]) -> Vec<Type> {
        let mut out = Vec::new();
        for e in effects.effect_items() {
            if let Some(b) = self.effect_bound(e, bounds) {
                out.extend(b.effect_items().iter().cloned());
            }
        }
        out
    }
}

/// `effects` but `unsafe`, which an `unsafe` block takes away.
fn without_unsafe(effects: &Type) -> Type {
    let items = effects.effect_items().iter();
    Type::effects(items.filter(|e| **e != Type::Unsafe).cloned())
}

/// That what's at a site, `what`, has effects `missing`: "`f` may panic".
/// A use of a static says what it is, "`COUNT` is a static", which is all
/// there is to say of its `global`.
pub(super) fn says(what: &str, missing: &Type) -> String {
    match what.ends_with(" is a static") {
        true => what.to_string(),
        false => format!("{what} {}", does(missing)),
    }
}

/// What effects `missing` mean a call may do: "may panic".
pub(super) fn does(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "may panic".into(),
        [Type::Alloc] => "may allocate".into(),
        [Type::Alloc, Type::Panic] => "may allocate and panic".into(),
        [Type::Unsafe] => "is unsafe".into(),
        [Type::Global] => "may use a static".into(),
        _ => format!("may have the effects `{}`", effects_text(missing)),
    }
}

/// What a function type without effects `missing` can't do.
fn cant(missing: &Type) -> String {
    match missing.effect_items() {
        [Type::Panic] => "can't panic".into(),
        [Type::Alloc] => "can't allocate".into(),
        [Type::Unsafe] => "can't be unsafe".into(),
        [Type::Global] => "can't use a static".into(),
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

/// The fix that writes `effects`, inferred, as `f`'s: `with {}` if there
/// are none, which then says so.
pub(super) fn inferred_fix(f: &FnDecl, effects: &Type) -> Fix {
    let text = written_with(f, effects);
    Fix::new(
        format!("write its effects: `with {text}`"),
        vec![Edit {
            span: effects_span(f),
            text: format!(" with {text}"),
        }],
    )
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
    let written = f.effects.iter().flat_map(|e| &e.items);
    let mut items: Vec<String> = written.clone().map(Effect::to_string).collect();
    // Each by what it is, without its condition, which it keeps.
    let mut atoms: Vec<String> = (written.cloned())
        .map(|mut e| {
            e.cond = None;
            e.to_string()
        })
        .collect();
    for e in all.effect_items() {
        let text = e.to_string();
        if !atoms.contains(&text) {
            atoms.push(text.clone());
            items.push(text);
        }
    }
    match items.len() {
        1 => items.remove(0),
        _ => format!("{{{}}}", items.join(", ")),
    }
}
