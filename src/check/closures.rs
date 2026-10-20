//! Closures that capture, as values of types of their own (see
//! docs/06c-closures.md).
//!
//! A closure that captures is a struct of what it captures, with an impl of
//! the `Fn` traits whose body is the closure's: its type is a
//! `Type::Adt` named by its symbol, whose definition is a
//! `traits::ClosureDef`. How it captures each variable, and so which of the
//! traits it implements, depends on the types of what the body does with
//! them, so it's worked out once the function it's in has its types
//! (`Checker::settle_closures`), innermost closure first. Until then, what
//! it implements isn't asked (`Checker::oblige` waits).
//!
//! A variable is captured the least way that does what the body does with
//! it: by `&` if it's only read, by `&mut` if it's changed or borrowed
//! mutably, and by value if it's moved out of. `move` captures everything
//! by value. Then the closure is
//!
//! - `FnOnce` only, if its body moves out of what it captured by value, or
//!   it owns something that isn't `Discard`: a value that must be used up
//!   can only be used up once. Its body then has to (see `moves::linear`),
//!   so that whatever implements `FnMut` may be thrown away;
//! - `FnMut` if it changes what it captured, or borrows it mutably;
//! - `Fn` otherwise.
//!
//! The borrow checker checks the body as a function taking the closure by
//! `&`, `&mut` or by value, as its kind says, and each capture is a field
//! of that: so a closure can't give out what it holds by `&mut` twice, or
//! what it owns at all (see `borrowck::closure`).

use super::Checker;
use super::typed::Res;
use crate::ast::*;
use crate::error::Error;
use crate::traits::{FnKind, Pred};
use crate::types::{FnType, Region, Type};
use std::collections::HashMap;
use std::sync::Arc;

/// A closure in the function being checked whose captures, if it has any,
/// or the lifetimes of whose result, are to be worked out at its end.
pub(super) struct Pending {
    /// Its symbol, if it captures: then its type is one of its own.
    pub(super) symbol: Option<String>,
    pub(super) lambda: Arc<Lambda>,
    /// The variables it captures, by id, in the order it names them.
    pub(super) captured: Vec<NodeId>,
    /// Its type, as the checker made it.
    pub(super) ty: Type,
    /// The names of its result's lifetimes still to work out
    /// (`result_lifetime`).
    pub(super) later: Vec<String>,
}

/// The name of the `i`th lifetime of closure `l`'s result, until it's
/// worked out from its body (`Checker::decide_result`). No program can
/// write it, and each closure's are its own.
pub(super) fn result_lifetime(l: &Lambda, i: usize) -> String {
    format!("'?r{}.{i}", l.body.span.start)
}

/// Is `r` a lifetime of a closure's result still to work out?
pub(crate) fn is_result_lifetime(r: &Region) -> bool {
    matches!(r, Region::Named(n) if n.starts_with("'?r"))
}

/// How a closure's body uses a variable, the most it needs: read it
/// through a `&`, through a `&mut`, or move it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Use {
    Read,
    /// Changes what it points to, if it's a reference.
    Unique,
    Mutate,
    Move,
}

impl Checker {
    /// The type of closure `l`, whose signature is `sig`, if it captures
    /// the variables `captured`, by id: one of its own, generic over every
    /// type parameter in scope.
    pub(super) fn closure_type(
        &mut self,
        l: &Arc<Lambda>,
        captured: Vec<NodeId>,
        sig: FnType,
        later: Vec<String>,
    ) -> Type {
        let symbol = format!("{{closure#{}}}", l.body.span.start);
        let def = crate::traits::ClosureDef {
            params: self.type_params.clone(),
            lifetimes: Vec::new(),
            sig,
            kind: None,
            fields: Vec::new(),
        };
        self.adts.insert(
            symbol.clone(),
            crate::types::AdtDef {
                params: def.params.clone(),
                lifetimes: Vec::new(),
                bounds: Vec::new(),
                preds: Vec::new(),
                eqs: Vec::new(),
                kind: crate::types::AdtKind::Struct(Vec::new()),
            },
        );
        self.adts.proj.closures.insert(symbol.clone(), def.clone());
        self.traits.closures.insert(symbol.clone(), def);
        let args = self.type_params.iter().cloned().map(Type::Param).collect();
        let ty = Type::Adt(symbol.clone(), Vec::new(), args);
        self.closures.push(Pending {
            symbol: Some(symbol),
            lambda: l.clone(),
            captured,
            ty: ty.clone(),
            later,
        });
        ty
    }

    /// What calls of type variable `v` take and give, if a bound on what
    /// it's for says, for a closure with `n` parameters: from the goal
    /// `<_ as FnOnce>::(signature)` = `fn(..) -> ..` that a call's bound
    /// `F: Fn(..) -> ..` makes, with the lifetimes it's generic over.
    pub(super) fn wanted_sig(&self, v: u32, n: usize) -> Option<FnType> {
        self.goals
            .iter()
            .find_map(|g| match (&g.proj, self.shallow(&g.is)) {
                (Type::Proj(s, _, name), Type::Fn(sig))
                    if name == crate::traits::SIG
                        && self.shallow(s) == Type::Var(v)
                        && sig.params.len() == n =>
                {
                    Some(sig)
                }
                _ => None,
            })
    }

    /// Is `t` a closure type whose captures, or its result's lifetimes,
    /// aren't worked out yet, or does it have one in it? Then what it
    /// implements can't be asked yet: whether its signature is what a bound
    /// says depends on them.
    pub(super) fn has_unsettled_closure(&self, t: &Type) -> bool {
        let mut found = false;
        super::each_adt(t, &mut |name, _| {
            found |= self
                .traits
                .closures
                .get(name)
                .is_some_and(|c| c.kind.is_none());
        });
        found
            || self
                .decided_regions(t)
                .regions()
                .into_iter()
                .any(is_result_lifetime)
    }

    /// `t` with each lifetime of a closure's result that's been worked out
    /// what it turned out to be.
    pub(super) fn decided_regions(&self, t: &Type) -> Type {
        if self.decided.is_empty() {
            return t.clone();
        }
        t.map_regions(&mut |r| match r {
            Region::Named(n) => self.decided.get(n).cloned().unwrap_or_else(|| r.clone()),
            r => r.clone(),
        })
    }

    /// At the end of a function, once its types are known: how each of its
    /// closures captures what it does, and so which `Fn` traits it
    /// implements, and the types of its fields; then how each call of what
    /// isn't a function type gets at what it calls; then what the lifetimes
    /// of each closure's result left to work out are, which a closure's body
    /// needs all of those to say. Innermost first, as an outer closure
    /// captures what an inner one does, and calls it.
    pub(super) fn settle_closures(&mut self) -> Result<(), Error> {
        let pending = std::mem::take(&mut self.closures);
        for p in &pending {
            if let Some(symbol) = &p.symbol {
                self.settle_closure(symbol, p)?;
            }
        }
        self.settle_calls()?;
        for p in &pending {
            if !p.later.is_empty() {
                self.settle_result(p)?;
            }
        }
        Ok(())
    }

    fn settle_closure(&mut self, symbol: &String, p: &Pending) -> Result<(), Error> {
        let l = &*p.lambda;
        let mut uses: HashMap<NodeId, Use> = HashMap::new();
        self.uses_in(&l.body, Use::Read, true, &p.captured, &mut uses);
        let mut fields = Vec::new();
        let mut modes = Vec::new();
        let mut kind = FnKind::Fn;
        let mut why = String::new();
        for id in &p.captured {
            let bind = &self.typed.binds[id];
            let (name, t) = (bind.name.clone(), self.solved(&bind.ty));
            let used = uses.get(id).copied().unwrap_or(Use::Read);
            let mode = match (l.moves, used) {
                (Some(_), _) | (None, Use::Move) => Capture::Value,
                (None, Use::Mutate) => Capture::Mut,
                (None, Use::Unique) => Capture::Unique,
                (None, Use::Read) => Capture::Ref,
            };
            // What the body does with it, and what the closure is then.
            let (k, reason) = match (mode, used) {
                (Capture::Value, Use::Move) => (FnKind::FnOnce, format!("it moves `{name}` out")),
                (Capture::Value, _) if !self.is_discard(&t) => (
                    FnKind::FnOnce,
                    format!("it owns `{name}`, which must be used up"),
                ),
                (_, Use::Mutate) => (FnKind::FnMut, format!("it changes `{name}`")),
                (_, Use::Unique) => (FnKind::FnMut, format!("it changes what `{name}` points to")),
                _ => (FnKind::Fn, String::new()),
            };
            if k > kind {
                kind = k;
                why = reason;
            }
            let field = match mode {
                Capture::Ref => Type::Ref(Region::Erased, false, Box::new(t)),
                Capture::Mut | Capture::Unique => Type::Ref(Region::Erased, true, Box::new(t)),
                Capture::Value => t,
            };
            fields.push((name.clone(), field));
            modes.push((*id, mode));
        }
        // Each region in its fields' types and its signature is a lifetime
        // parameter of its own, but for those of its result still to work
        // out, which are worked out once the rest is known.
        let mut lifetimes = Vec::new();
        let mut fresh = |r: &Region| {
            if is_result_lifetime(r) {
                return r.clone();
            }
            let name = format!("'_k{}", lifetimes.len());
            lifetimes.push(name.clone());
            Region::Named(name)
        };
        let fields: Vec<(String, Type)> = fields
            .into_iter()
            .map(|(n, t)| (n, t.map_regions(&mut fresh)))
            .collect();
        let def = &self.traits.closures[symbol];
        let sig = Type::Fn(def.sig.clone());
        let Type::Fn(sig) = self
            .decided_regions(&self.solved(&sig))
            .map_regions(&mut fresh)
        else {
            unreachable!("a signature is a function type")
        };
        let def = crate::traits::ClosureDef {
            params: def.params.clone(),
            lifetimes: lifetimes.clone(),
            sig,
            kind: Some(kind),
            fields,
        };
        self.closure_why.insert(symbol.clone(), why);
        self.define_closure(symbol, &def);
        self.info.captures.insert(
            l.id,
            modes
                .iter()
                .map(|(id, m)| (self.typed.binds[id].name.clone(), *m))
                .collect(),
        );
        self.typed.modes.insert(p.lambda.id, modes);
        Ok(())
    }

    /// Works out the lifetimes of closure `p`'s result left to work out,
    /// from its body (`borrowck::decide_result`).
    fn settle_result(&mut self, p: &Pending) -> Result<(), Error> {
        let ty = self.solution(&p.ty);
        let decided = crate::borrowck::decide_result(self, &p.lambda, &ty, &p.later)?;
        let Some(symbol) = &p.symbol else {
            self.decided.extend(p.later.iter().cloned().zip(decided));
            return Ok(());
        };
        let mut def = self.traits.closures[symbol].clone();
        // One fixed where it's made is a lifetime parameter of its own.
        let decided: HashMap<&String, Region> = p
            .later
            .iter()
            .zip(decided)
            .map(|(name, r)| match r {
                Region::Erased => {
                    let own = format!("'_k{}", def.lifetimes.len());
                    def.lifetimes.push(own.clone());
                    (name, Region::Named(own))
                }
                r => (name, r),
            })
            .collect();
        let sig = Type::Fn(def.sig.clone()).map_regions(&mut |r| match r {
            Region::Named(n) => decided.get(n).cloned().unwrap_or_else(|| r.clone()),
            r => r.clone(),
        });
        let Type::Fn(sig) = sig else {
            unreachable!("a signature is a function type")
        };
        def.sig = sig;
        self.define_closure(symbol, &def);
        Ok(())
    }

    /// Defines closure `symbol`'s struct, and what it implements, as `def`
    /// says.
    fn define_closure(&mut self, symbol: &str, def: &crate::traits::ClosureDef) {
        let adt = self.adts.get_mut(symbol).expect("the closure's struct");
        adt.lifetimes = def.lifetimes.clone();
        adt.kind = crate::types::AdtKind::Struct(def.fields.clone());
        self.adts
            .proj
            .closures
            .insert(symbol.to_string(), def.clone());
        self.traits.closures.insert(symbol.to_string(), def.clone());
    }

    /// Notes in `uses` what `e` does with the variables `captured`, as it's
    /// used as `how`. `value` says `e`'s value is what's used, rather than
    /// the place it names: a read of one that's `Copy` copies it, which
    /// only needs a `&`.
    fn uses_in(
        &self,
        e: &Expr,
        how: Use,
        value: bool,
        captured: &[NodeId],
        uses: &mut HashMap<NodeId, Use>,
    ) {
        // Using a value of `e`'s type: moving it, unless it's `Copy`.
        let ty = |e: &Expr| self.typed.exprs.get(&e.id).map(|t| self.solved(t));
        // A `&mut` used is reborrowed, as `Builder::use_place` does.
        let read = |e: &Expr| match ty(e) {
            Some(Type::Ref(_, true, _)) => Use::Unique,
            Some(t) if !self.is_copy(&t) => Use::Move,
            _ => Use::Read,
        };
        let mut go = |e: &Expr, how: Use, value: bool| self.uses_in(e, how, value, captured, uses);
        // A reference used as another is reborrowed: `&*r`, or `&mut *r`,
        // which needs `r` unique, not `mut`.
        if let Some(&(_, mutable)) = self.typed.coerced.get(&e.id) {
            let how = if mutable { Use::Unique } else { Use::Read };
            return self.place_uses(e, how, captured, uses);
        }
        let how = match (how, value) {
            (Use::Read, true) => read(e),
            (how, _) => how,
        };
        match &e.kind {
            ExprKind::Var(..) | ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _) => {
                self.place_uses(e, how, captured, uses)
            }
            ExprKind::Unary(UnOp::Ref, x) => go(x, Use::Read, false),
            ExprKind::Unary(UnOp::RefMut, x) => go(x, Use::Mutate, false),
            ExprKind::Assign(_, place, v) => {
                go(place, Use::Mutate, false);
                go(v, Use::Read, true);
            }
            // `==` borrows both sides.
            ExprKind::Binary(BinOp::Eq | BinOp::Ne, l, r) => {
                go(l, Use::Read, false);
                go(r, Use::Read, false);
            }
            ExprKind::MethodCall(x, _, _, args) => {
                // Through references, `r.push(1)` is `push(&mut *r, 1)`.
                let receiver = self.typed.methods.get(&e.id);
                match receiver.map(|(_, derefs, autoref)| (*derefs, *autoref)) {
                    Some((_, Some(false))) => go(x, Use::Read, false),
                    Some((0, Some(true))) => go(x, Use::Mutate, false),
                    Some((_, Some(true))) => go(x, Use::Unique, false),
                    _ => go(x, Use::Read, true),
                }
                args.iter().for_each(|a| go(a, Use::Read, true));
            }
            ExprKind::Call(f, args) => {
                let callee = ty(f);
                let kind = match &callee {
                    Some(Type::Fn(_)) | None => None,
                    Some(t) => self.call_kind(t),
                };
                match kind {
                    // Called through what it points to: `&*f` or `&mut *f`,
                    // or as it is: by `&`, by `&mut`, or taking it.
                    Some((_, FnKind::Fn)) => go(f, Use::Read, false),
                    Some((0, FnKind::FnMut)) => go(f, Use::Mutate, false),
                    Some((_, FnKind::FnMut)) => go(f, Use::Unique, false),
                    Some((_, FnKind::FnOnce)) | None => go(f, Use::Read, true),
                }
                args.iter().for_each(|a| go(a, Use::Read, true));
            }
            ExprKind::Match(scrutinee, arms) => {
                let moves = arms.iter().any(|a| self.pattern_moves(&a.pat, scrutinee));
                go(scrutinee, if moves { Use::Move } else { Use::Read }, false);
                arms.iter().for_each(|a| go(&a.body, Use::Read, true));
            }
            ExprKind::Block(stmts, tail) => {
                for s in stmts {
                    match s {
                        Stmt::Let {
                            pat,
                            init: Some(init),
                            ..
                        } => {
                            let moves = self.pattern_moves(pat, init);
                            go(init, if moves { Use::Move } else { Use::Read }, false);
                        }
                        Stmt::Expr(x) | Stmt::BlockLike(x) => go(x, Use::Read, true),
                        _ => {}
                    }
                }
                tail.iter().for_each(|t| go(t, Use::Read, true));
            }
            // An inner closure captures what it does from here: the same
            // way, but a copy of a `Copy` value only needs a `&`.
            ExprKind::Lambda(l) => {
                let Some(modes) = self.typed.modes.get(&l.id) else {
                    return;
                };
                for (id, mode) in modes {
                    if !captured.contains(id) {
                        continue;
                    }
                    let t = self.solved(&self.typed.binds[id].ty);
                    let used = match mode {
                        Capture::Ref => Use::Read,
                        Capture::Mut => Use::Mutate,
                        Capture::Unique => Use::Unique,
                        Capture::Value if self.is_copy(&t) => Use::Read,
                        Capture::Value => Use::Move,
                    };
                    note(uses, *id, used);
                }
            }
            _ => e
                .children()
                .into_iter()
                .for_each(|c| go(c, Use::Read, true)),
        }
    }

    /// `uses_in` for an expression used as a place, `how`: a variable,
    /// what's at a field of one, or what one points to.
    fn place_uses(&self, e: &Expr, how: Use, captured: &[NodeId], uses: &mut HashMap<NodeId, Use>) {
        match &e.kind {
            ExprKind::Var(..) => {
                if let Some(Res::Local(id)) = self.typed.vars.get(&e.id)
                    && captured.contains(id)
                {
                    note(uses, *id, how);
                }
            }
            // Changing what's through a reference only needs the reference
            // to be unique, not `mut`: a field through one, or `*r`.
            ExprKind::Field(x, _) | ExprKind::Unary(UnOp::Deref, x) => {
                let through = match &e.kind {
                    ExprKind::Field(..) => self
                        .typed
                        .fields
                        .get(&e.id)
                        .is_some_and(|(derefs, _)| *derefs > 0),
                    _ => true,
                };
                let reference = self
                    .typed
                    .exprs
                    .get(&x.id)
                    .is_some_and(|t| matches!(self.solved(t), Type::Ref(..)));
                let how = match how {
                    Use::Mutate if through && reference => Use::Unique,
                    how => how,
                };
                self.place_uses(x, how, captured, uses)
            }
            _ => self.uses_in(e, how, false, captured, uses),
        }
    }

    /// Does matching `pat` against `scrutinee` move out of it: does it bind
    /// a value that isn't `Copy` by value, or take apart one that isn't
    /// `Discard`, which uses it up?
    fn pattern_moves(&self, pat: &Pattern, scrutinee: &Expr) -> bool {
        let Some(t) = self.typed.exprs.get(&scrutinee.id) else {
            return true;
        };
        let t = self.solved(t);
        if matches!(t, Type::Ref(..)) {
            return false;
        }
        let takes_apart = !matches!(pat.kind, PatternKind::Wild | PatternKind::Bind(..));
        if takes_apart && !self.is_discard(&t) {
            return true;
        }
        let mut moves = false;
        pattern_sites(pat, &mut |site| {
            let Some(id) = self.typed.sites.get(&site) else {
                return;
            };
            let bind = &self.typed.binds[id];
            if bind.by_ref.is_none() && !self.is_copy(&self.solved(&bind.ty)) {
                moves = true;
            }
        });
        moves
    }

    /// How a call of `t`, which isn't a function type, gets at what's
    /// called: through how many references, and by which of the `Fn`
    /// traits. `None` if it can't be called.
    pub(super) fn call_kind(&self, t: &Type) -> Option<(usize, FnKind)> {
        match self.solved(t) {
            Type::Fn(_) => Some((0, FnKind::Fn)),
            Type::Ref(_, m, inner) => {
                let (derefs, kind) = self.call_kind(&inner)?;
                match (m, kind) {
                    (false, FnKind::Fn) => Some((derefs + 1, FnKind::Fn)),
                    (true, FnKind::Fn | FnKind::FnMut) => Some((derefs + 1, FnKind::FnMut)),
                    _ => None,
                }
            }
            Type::Adt(symbol, ..) => Some((0, self.traits.closures.get(&symbol)?.kind?)),
            // What its bounds say: the strongest of the `Fn` traits they do.
            t @ (Type::Param(_) | Type::Proj(..) | Type::Opaque(..)) => {
                let env = (&self.env, &self.adts.proj.local);
                [FnKind::Fn, FnKind::FnMut, FnKind::FnOnce]
                    .into_iter()
                    .find(|k| self.traits.implements(&t, k.name(), env.0, env.1).is_ok())
                    .map(|k| (0, k))
            }
            _ => None,
        }
    }

    /// At the end of a function, once its closures are settled: how each of
    /// its calls of what isn't a function type gets at what it calls.
    pub(super) fn settle_calls(&mut self) -> Result<(), Error> {
        for (e, t, span) in std::mem::take(&mut self.calls) {
            let t = self.solved(&t);
            if super::unify::has_vars(&t) {
                continue;
            }
            let Some(call) = self.call_kind(&t) else {
                return Err(Error::ty(
                    span,
                    format!("this can't be called through a reference: `{t}` can be only once"),
                ));
            };
            self.typed.calls.insert(e, call);
            self.info.calls.insert(e, call);
        }
        Ok(())
    }

    /// Why a closure type, if `t` is one, implements only the `Fn` traits
    /// it does, for a message: "it moves `b` out".
    pub(super) fn closure_why(&self, t: &Type) -> Option<(FnKind, &str)> {
        let Type::Adt(symbol, ..) = t else {
            return None;
        };
        let kind = self.traits.closures.get(symbol)?.kind?;
        Some((kind, self.closure_why.get(symbol)?.as_str()))
    }

    /// The obligation for a callable `t` to be called, at `span`: the
    /// trait its calls need.
    pub(super) fn callable_pred(t: &Type) -> Pred {
        Pred {
            ty: t.clone(),
            trait_name: "FnOnce".into(),
        }
    }
}

/// Notes that the variable `id` is used `how`, if that's more than it was.
fn note(uses: &mut HashMap<NodeId, Use>, id: NodeId, how: Use) {
    let u = uses.entry(id).or_insert(how);
    *u = (*u).max(how);
}

/// Calls `f` with where each variable `p` binds is bound.
fn pattern_sites(p: &Pattern, f: &mut impl FnMut(NodeId)) {
    match &p.kind {
        PatternKind::Bind(..) => f(p.id),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().for_each(|p| pattern_sites(p, f))
        }
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().for_each(|fp| pattern_sites(&fp.pat, f))
        }
        _ => {}
    }
}
