//! Refinement types: `{ d: int | d != 0 }`, the `int`s that aren't 0 (see
//! docs/08-refinement-types.md and docs/08.1-field-refinements.md).
//!
//! A refinement is the whole type of a parameter, a result, a `let` or a
//! struct's field, never part of another type, and only of an `int` or a
//! `bool`. It isn't in `Type` at all: unification, traits and
//! monomorphisation never see one. What it says is checked by a pass of its
//! own over each body, once its types are known (`Checker::refine`), which
//! keeps track of what's known at each point (*facts*) and proves from them
//! what each refinement needs, with the procedure in `refine/logic.rs`:
//!
//! - an argument has its parameter's refinement, a result its function's,
//!   and a value given to a refined `let`, or assigned to one, its;
//! - a struct's literal has its fields' refinements, and so does it after
//!   an assignment to a field one of them is about;
//! - an operator can't overflow or divide by zero: one proved not to has
//!   no `panic`, which is how refinements make arithmetic total.
//!
//! Facts come from refined parameters and `let`s, from the refinements of
//! the fields of every struct value, from what calls of refined functions
//! return, from the conditions of `if`, `while`, `&&`, `||` and the
//! patterns of a `match` on an `int`, from what a `let` or an assignment
//! gives a *place* (a variable, or a field or `*` of one: `self.start`),
//! and from operators that returned, as they'd have panicked otherwise.
//! They're about *values*: each time a place gets a new one, it's a new
//! variable of the logic (*static single assignment*), so a fact, once
//! true, stays true, and only what a place stands for changes.
//!
//! Soundness: every fact in the context holds of the values the program
//! has at that point, which is an induction on how the program runs. A fact
//! is added only where it's been proved, where a condition is true, after
//! an operator returned, or of a struct value, which has its fields'
//! refinements (below). A place stands for a new variable of the logic
//! whenever it may change: when it, or what it's in, is assigned; at the
//! start of each time round a loop that assigns it, or lends it to a call;
//! after a branch that does, where it's whichever value the branch taken
//! gave it; and after a call it's lent to `&mut` (`Walk::borrows`), when
//! nothing can keep the reference after the call. A place that may change
//! some other way, through a `&mut` that may outlive where it's made, or a
//! closure that captures it, gives a new value each time it's read: nothing
//! is known of it but its refinements, which every assignment, inside a
//! closure too, must keep (`Walk::unstable_in`). A `&` can't change what it
//! points to, as nothing in T can be changed through one, so `*r` for a
//! `let r = &p;` is `p` (`Walk::alias`). A refined variable can't be
//! borrowed `&mut` at all, nor a field a refinement is about, so the
//! refinements of a struct's fields hold of every value of it, wherever it
//! is: every way of making or changing one checks them, and anything else,
//! generic code included, only moves whole values. A refinement's predicate
//! names only what can't change. What a call returns has its function's
//! refinement by induction on how deep the calls go, as for effects: a call
//! that never returns gives nothing to count on. And what's called through
//! a value can't have refined parameters, nor a trait's method, so every
//! call that skips a check is of a function with nothing to check.

mod logic;

use super::Checker;
use super::typed::Res;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{Region, Type, all_fields, struct_fields};
use logic::{Formula, Kind, Lin, Var};
use std::collections::{HashMap, HashSet};

/// The predicate of a refinement, whose names are in `Checker::pred_names`.
#[derive(Debug, Clone)]
pub(super) struct Refinement {
    pred: Expr,
}

impl Refinement {
    fn text(&self) -> String {
        crate::pretty::expr_text(&self.pred)
    }
}

/// The refinements in a function's signature, by symbol (`Checker::refined`).
#[derive(Debug, Clone)]
pub(super) struct RefinedSig {
    name: String,
    /// Its parameters' names.
    names: Vec<String>,
    params: Vec<Option<Refinement>>,
    ret: Option<Refinement>,
}

/// The refinements of a struct's fields, by the struct's symbol
/// (`Checker::field_refined`). They're kept by every value of it.
#[derive(Debug, Clone)]
pub(super) struct RefinedFields {
    /// The struct's name.
    name: String,
    /// Its fields' names.
    names: Vec<String>,
    refs: Vec<Option<Refinement>>,
    /// The fields each field's refinement names, other than itself.
    named: Vec<Vec<usize>>,
}

impl RefinedFields {
    /// The refined fields whose refinements are about field `i`: its own,
    /// and those that name it.
    fn about(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.refs.len())
            .filter(move |&j| self.refs[j].is_some() && (j == i || self.named[j].contains(&i)))
    }

    /// Why field `i` can't be borrowed `&mut`, if it can't.
    fn constrained(&self, i: usize) -> Option<String> {
        let j = self.about(i).next()?;
        Some(match j == i {
            true => format!(
                "field `{}` of `{}` has a refinement",
                self.names[i], self.name
            ),
            false => format!(
                "the refinement of field `{}` of `{}` names it",
                self.names[j], self.name
            ),
        })
    }
}

/// What a name in a predicate is.
#[derive(Debug, Clone, Copy)]
pub(super) enum PredName {
    /// The refinement's own, the value refined.
    Binder,
    /// A field of the struct, by index, in a field's refinement.
    Field(usize),
    /// The function's parameter, by index.
    Param(usize),
    /// A local variable, by id.
    Local(NodeId),
}

/// What a name a predicate is checked in names: a value, of this type,
/// declared there, or a `mut` variable.
enum Named {
    Value(Type, PredName, Span),
    Mut,
    /// One it can't name, and why.
    Not(String),
}

impl Checker {
    /// Checks the refinements in the signature of `f`, whose parameters
    /// and result have the types `params` and `ret` without them, and
    /// keeps them. `allowed` is false in a trait's methods and an impl's
    /// for a trait, which can't have any.
    pub(super) fn refined_signature(
        &mut self,
        f: &FnDecl,
        params: &[Type],
        ret: &Type,
        allowed: bool,
    ) -> Result<(), Error> {
        let written = f.params.iter().map(|p| &p.ty).chain([&f.ret]);
        let Some(first) = written.clone().find(|t| t.refinement().is_some()) else {
            return Ok(());
        };
        if !allowed {
            return Err(Error::ty(
                first.span,
                "refinements can't be in a trait's methods yet: an impl's would have to ask no \
                 more than the trait's, and promise no less",
            ));
        }
        let mut sig = RefinedSig {
            name: f.name.name.clone(),
            names: f.params.iter().map(|p| p.name.name.clone()).collect(),
            params: Vec::new(),
            ret: None,
        };
        for (i, p) in f.params.iter().enumerate() {
            let scope = |name: &str| {
                f.params[..i]
                    .iter()
                    .position(|q| q.name.name == name)
                    .map(|j| match f.params[j].mutable {
                        Some(_) => Named::Mut,
                        None => Named::Value(
                            params[j].clone(),
                            PredName::Param(j),
                            f.params[j].name.span,
                        ),
                    })
            };
            let r = self.refinement(&p.ty, &params[i], &scope)?;
            sig.params.push(r);
        }
        let scope = |name: &str| {
            f.params
                .iter()
                .position(|q| q.name.name == name)
                .map(|j| match f.params[j].mutable {
                    Some(_) => Named::Mut,
                    None => {
                        Named::Value(params[j].clone(), PredName::Param(j), f.params[j].name.span)
                    }
                })
        };
        sig.ret = self.refinement(&f.ret, ret, &scope)?;
        let symbol = self.sig_owner.0.clone();
        self.refined.insert(symbol, sig);
        Ok(())
    }

    /// Checks the refinements of the fields of struct `s`, whose symbol
    /// is `symbol`, and keeps them. A field's names the fields before it,
    /// as visible as it is.
    pub(super) fn field_refinements(&mut self, s: &StructDecl, symbol: &str) -> Result<(), Error> {
        if s.fields.iter().all(|f| f.ty.refinement().is_none()) {
            return Ok(());
        }
        let ty = Type::Adt(symbol.to_string(), Vec::new(), Vec::new());
        let types: Vec<Type> = match self.adts.get(symbol).map(|d| &d.kind) {
            Some(crate::types::AdtKind::Struct(fields)) => {
                fields.iter().map(|(_, t)| t.clone()).collect()
            }
            _ => unreachable!("struct `{ty}` collected"),
        };
        let mut fields = RefinedFields {
            name: s.name.name.clone(),
            names: s.fields.iter().map(|f| f.name.name.clone()).collect(),
            refs: Vec::new(),
            named: Vec::new(),
        };
        for (i, f) in s.fields.iter().enumerate() {
            let public = |j: usize| s.fields[j].public.is_some();
            let scope = |name: &str| {
                let j = s.fields.iter().position(|g| g.name.name == name)?;
                let is = |p: bool| if p { "is" } else { "isn't" };
                let (binder, _) = f.ty.refinement()?;
                Some(if j == i {
                    Named::Not(format!(
                        "a field's refinement names the field's own value by its binder, `{}`, \
                         not `{name}`",
                        binder.name
                    ))
                } else if j > i {
                    Named::Not(format!(
                        "a field's refinement can only name the fields before it, and `{name}` \
                         comes after `{}`",
                        f.name.name
                    ))
                } else if public(j) != public(i) {
                    Named::Not(format!(
                        "the refinement of `{}`, which {} `pub`, can't name `{name}`, which {}: a \
                         field's refinement can only name fields as visible as it",
                        f.name.name,
                        is(public(i)),
                        is(public(j)),
                    ))
                } else {
                    Named::Value(types[j].clone(), PredName::Field(j), s.fields[j].name.span)
                })
            };
            let r = self.refinement(&f.ty, &types[i], &scope)?;
            let mut named = Vec::new();
            if let Some(r) = &r {
                each_expr(&r.pred, &mut |e| {
                    if let Some(PredName::Field(j)) = self.pred_names.get(&e.id) {
                        named.push(*j);
                    }
                });
            }
            fields.refs.push(r);
            fields.named.push(named);
        }
        self.field_refined.insert(symbol.to_string(), fields);
        Ok(())
    }

    /// Checks the refinement of a `let` of `pat`, of type `t` (the type
    /// without it), written `ann`, in the scope of the locals so far.
    pub(super) fn let_refinement(
        &mut self,
        pat: &Pattern,
        ann: &TypeExpr,
        t: &Type,
        init: bool,
    ) -> Result<(), Error> {
        if ann.refinement().is_none() {
            return Ok(());
        }
        if !matches!(pat.kind, PatternKind::Bind(..)) {
            return Err(Error::ty(
                ann.span,
                "a refinement can only be the type of a `let` of one variable",
            ));
        }
        if !init {
            return Err(Error::ty(
                ann.span,
                "a `let` with a refinement needs a value, which must have it",
            ));
        }
        let locals: Vec<(String, Type, bool, NodeId, Span)> = self.locals[self.barrier..]
            .iter()
            .map(|l| (l.name.clone(), l.ty.clone(), l.mutable, l.id(), l.def()))
            .collect();
        let scope = |name: &str| {
            locals
                .iter()
                .rev()
                .find(|l| l.0 == name)
                .map(|l| match l.2 {
                    true => Named::Mut,
                    false => Named::Value(l.1.clone(), PredName::Local(l.3), l.4),
                })
        };
        let r = self.refinement(ann, t, &scope)?.expect("refined");
        // What it names, it uses.
        let named: HashSet<NodeId> = (self.pred_names.values())
            .filter_map(|n| match n {
                PredName::Local(id) => Some(*id),
                _ => None,
            })
            .collect();
        for l in &mut self.locals[self.barrier..] {
            if named.contains(&l.id()) {
                l.used = true;
            }
        }
        let bind = pat.id;
        self.refined_lets.insert(bind, r);
        Ok(())
    }

    /// The refinement written as `t`, whose base type is `base`, if it's
    /// one, checked: its base is an `int` or a `bool`, and its predicate a
    /// `bool` in the logic, naming what `scope` says.
    fn refinement(
        &mut self,
        t: &TypeExpr,
        base: &Type,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<Option<Refinement>, Error> {
        let Some((binder, pred)) = t.refinement() else {
            return Ok(None);
        };
        let base = self.solved(base);
        if !matches!(base, Type::Int | Type::Bool) {
            return Err(Error::ty(
                t.unrefined().span,
                format!("only `int` and `bool` can be refined, not `{base}`"),
            ));
        }
        let kind = super::NoteKind::Var(binder.name.clone());
        self.note(binder.span, kind, Some(base.clone()), Some(binder.span));
        let scope = |name: &str| match name == binder.name {
            true => Some(Named::Value(base.clone(), PredName::Binder, binder.span)),
            false => scope(name),
        };
        let got = self.pred_type(pred, &scope)?;
        if got != Type::Bool {
            return Err(Error::ty(
                pred.span,
                format!("expected `bool`, found `{got}`"),
            ));
        }
        Ok(Some(Refinement { pred: pred.clone() }))
    }

    /// The type of predicate (part) `e`, which must be in the logic:
    /// linear arithmetic on `int`s, compared, and `bool`s.
    fn pred_type(
        &mut self,
        e: &Expr,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<Type, Error> {
        let want = |cx: &mut Checker, e: &Expr, t: Type| -> Result<(), Error> {
            let got = cx.pred_type(e, scope)?;
            if got != t {
                return Err(Error::ty(e.span, format!("expected `{t}`, found `{got}`")));
            }
            Ok(())
        };
        match &e.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Var(name, targs) if targs.is_empty() => match scope(name) {
                Some(Named::Value(t, what, def)) => {
                    let t = self.solved(&t);
                    if !matches!(t, Type::Int | Type::Bool) {
                        return Err(Error::ty(
                            e.span,
                            format!(
                                "a refinement can only name `int`s and `bool`s, and `{name}` is \
                                 a `{t}`"
                            ),
                        ));
                    }
                    self.pred_names.insert(e.id, what);
                    let kind = match what {
                        PredName::Field(_) => super::NoteKind::Field(name.clone()),
                        _ => super::NoteKind::Var(name.clone()),
                    };
                    self.note(e.span, kind, Some(t.clone()), Some(def));
                    Ok(t)
                }
                Some(Named::Not(why)) => Err(Error::ty(e.span, why)),
                Some(Named::Mut) => Err(Error::ty(
                    e.span,
                    format!(
                        "a refinement can't name `{name}`, which is `mut`: it would be unclear \
                         which of its values it meant"
                    ),
                )),
                None => Err(Error::ty(
                    e.span,
                    format!("cannot find value `{name}` in this refinement"),
                )),
            },
            ExprKind::Unary(UnOp::Neg, x) => want(self, x, Type::Int).map(|()| Type::Int),
            ExprKind::Unary(UnOp::Not, x) => want(self, x, Type::Bool).map(|()| Type::Bool),
            ExprKind::Binary(op, l, r) => match op {
                BinOp::Add | BinOp::Sub => {
                    want(self, l, Type::Int)?;
                    want(self, r, Type::Int)?;
                    Ok(Type::Int)
                }
                BinOp::Mul => {
                    want(self, l, Type::Int)?;
                    want(self, r, Type::Int)?;
                    if !constant(l) && !constant(r) {
                        return Err(Error::ty(
                            e.span,
                            format!(
                                "a refinement can only multiply by a constant: `{}` isn't linear",
                                crate::pretty::expr_text(e)
                            ),
                        ));
                    }
                    Ok(Type::Int)
                }
                BinOp::Div | BinOp::Rem => Err(Error::ty(
                    e.span,
                    format!(
                        "a refinement can't use `{op}` yet: only `+`, `-` and `*` by a constant"
                    ),
                )),
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    want(self, l, Type::Int)?;
                    want(self, r, Type::Int)?;
                    Ok(Type::Bool)
                }
                BinOp::Eq | BinOp::Ne => {
                    let t = self.pred_type(l, scope)?;
                    want(self, r, t)?;
                    Ok(Type::Bool)
                }
                BinOp::And | BinOp::Or => {
                    want(self, l, Type::Bool)?;
                    want(self, r, Type::Bool)?;
                    Ok(Type::Bool)
                }
            },
            _ => Err(Error::ty(
                e.span,
                "a refinement can only use literals, names, `+`, `-`, `*` by a constant, \
                 comparisons, `&&`, `||` and `!`",
            )),
        }
    }

    /// Checks what the refinements in body `f` need, and works out which
    /// of its operators can't panic, taking those out of its effects.
    pub(super) fn refine(&mut self, f: &FnDecl) -> Result<(), Error> {
        let id = self.current.expect("in a function");
        let symbol = self.fns[id.0].symbol.clone();
        let sig = self.refined.get(&symbol).cloned();
        let mut w = Walk {
            cx: self,
            kinds: Vec::new(),
            facts: Vec::new(),
            reachable: true,
            cur: HashMap::new(),
            invariants: HashMap::new(),
            unstable: HashSet::new(),
            short: HashMap::new(),
            alias: HashMap::new(),
            declared: Vec::new(),
            proved: HashSet::new(),
            params: f.params.iter().map(|p| p.name.id).collect(),
            ret: None,
            closures: 0,
            loops: Vec::new(),
        };
        w.unstable_in(&f.body, false);
        let mut ret = None;
        for (i, p) in f.params.iter().enumerate() {
            if let Some(r) = sig.as_ref().and_then(|s| s.params[i].clone()) {
                w.invariants.insert(p.name.id, (r, p.name.name.clone()));
            }
            w.bind(p.name.id, None);
        }
        if let Some(sig) = &sig
            && let Some(r) = &sig.ret
        {
            ret = Some((r.clone(), sig.name.clone()));
        }
        w.ret = ret;
        let v = w.expr(&f.body)?;
        w.result(v, super::expr::result_span(&f.body))?;
        let proved = std::mem::take(&mut w.proved);
        let keep = |node: Option<NodeId>| node.is_none_or(|n| !proved.contains(&n));
        self.effect_sites.retain(|s| keep(s.node));
        self.withins.retain(|w| keep(w.node));
        self.info.proved.extend(proved);
        Ok(())
    }
}

/// Calls `f` with `e` and everything in it.
fn each_expr(e: &Expr, f: &mut impl FnMut(&Expr)) {
    f(e);
    for c in e.children() {
        each_expr(c, f);
    }
}

/// Calls `f` with each binding in `pat`.
fn each_bind(pat: &Pattern, f: &mut impl FnMut(&Pattern)) {
    match &pat.kind {
        PatternKind::Bind(..) => f(pat),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().for_each(|p| each_bind(p, f))
        }
        PatternKind::Struct(_, _, fields, _) => fields.iter().for_each(|fp| each_bind(&fp.pat, f)),
        PatternKind::Move(p) => each_bind(p, f),
        PatternKind::Wild
        | PatternKind::Int(_)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}

/// Is `e` a constant: no names in it?
fn constant(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Unary(UnOp::Neg, x) => constant(x),
        ExprKind::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul, l, r) => constant(l) && constant(r),
        _ => false,
    }
}

/// What an expression is, to the logic: an `int`, as a linear sum, a
/// `bool`, as a formula, a struct, as its fields' values, or anything else,
/// of which nothing is known.
#[derive(Debug, Clone)]
enum Val {
    Int(Lin),
    Bool(Formula),
    Struct(Vec<Val>),
    Other,
}

/// Where a value is, a *place*: a local variable, and the fields and `*`s
/// from it to there. `self.start`, in a method taking `&mut self`, is the
/// field `start` of `*self`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Place {
    root: NodeId,
    path: Vec<Proj>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Proj {
    /// A struct's field, by index.
    Field(usize),
    /// What a reference or a box points to.
    Deref,
}

impl Place {
    fn local(id: NodeId) -> Place {
        Place {
            root: id,
            path: Vec::new(),
        }
    }

    fn then(&self, p: Proj) -> Place {
        let mut q = self.clone();
        q.path.push(p);
        q
    }

    /// Is it `outer`, or inside it?
    fn within(&self, outer: &Place) -> bool {
        self.root == outer.root && self.path.starts_with(&outer.path)
    }

    /// The field it is of the place it's in, if it's one.
    fn field_of(&self) -> Option<(Place, usize)> {
        match self.path.last()? {
            Proj::Field(i) => {
                let outer = Place {
                    root: self.root,
                    path: self.path[..self.path.len() - 1].to_vec(),
                };
                Some((outer, *i))
            }
            Proj::Deref => None,
        }
    }
}

/// What an argument passes (see `Walk::passed`).
struct Passed {
    borrowed: Option<Place>,
    ty: Type,
    /// Is it passed as a `&`, whatever its type?
    shared: bool,
}

/// The state to run each branch from.
struct Snap {
    facts: usize,
    cur: HashMap<Place, Var>,
    reachable: bool,
    declared: usize,
}

/// How a path through a branch ended: what it learned, what each place
/// stood for at its end, whether it gets there, and its value.
struct Outcome {
    facts: Vec<Formula>,
    cur: HashMap<Place, Var>,
    reachable: bool,
    val: Val,
}

/// A loop around what's being walked: its label, and whether a `break`
/// leaves it.
struct Loop {
    label: Option<String>,
    broke: bool,
}

/// The pass over one body (see `Checker::refine`).
struct Walk<'c> {
    cx: &'c Checker,
    /// What each variable of the logic stands for, by number.
    kinds: Vec<Kind>,
    /// What's known here, on this path.
    facts: Vec<Formula>,
    /// Whether this point can be reached at all.
    reachable: bool,
    /// Which variable of the logic each `int` or `bool` place stands for
    /// here. One that isn't in it is a value nothing's known of yet, but
    /// its refinements.
    cur: HashMap<Place, Var>,
    /// The refinement of each refined local, with its name.
    invariants: HashMap<NodeId, (Refinement, String)>,
    /// The places that may change other than by assignment, with all
    /// that's in them.
    unstable: HashSet<Place>,
    /// The places each call borrows `&mut` only while it runs, by the
    /// call's node: what's known of them is forgotten when it returns.
    short: HashMap<NodeId, Vec<Place>>,
    /// What each local bound to a `&` of a place points to: `*r` is that
    /// place, which can't change while `r` can be used.
    alias: HashMap<NodeId, Place>,
    /// The locals in scope, in the order they were bound.
    declared: Vec<NodeId>,
    /// The operators proved not to panic, by node.
    proved: HashSet<NodeId>,
    /// The ids of the function's parameters.
    params: Vec<NodeId>,
    /// The refinement of the function's result, with its name.
    ret: Option<(Refinement, String)>,
    /// How many closures deep this is.
    closures: usize,
    loops: Vec<Loop>,
}

/// How a predicate's names other than its binder get their values: those
/// of the function being walked, a call's arguments, or the fields of a
/// struct.
#[derive(Clone, Copy)]
enum Env<'a> {
    Here,
    Call(&'a [Val]),
    Fields(&'a [Val]),
}

impl Walk<'_> {
    fn fresh(&mut self, kind: Kind) -> Var {
        self.kinds.push(kind);
        (self.kinds.len() - 1) as Var
    }

    /// Variable `v` of the logic, as a value.
    fn var(&self, v: Var) -> Val {
        match self.kinds[v as usize] {
            Kind::Int => Val::Int(Lin::var(v)),
            Kind::Bool => Val::Bool(Formula::holds(v)),
        }
    }

    /// A value of type `t` of which nothing is known: but if it's a
    /// struct, its fields have its refinements.
    fn unknown(&mut self, t: &Type) -> Val {
        match t {
            Type::Int => Val::Int(Lin::var(self.fresh(Kind::Int))),
            Type::Bool => Val::Bool(Formula::holds(self.fresh(Kind::Bool))),
            Type::Adt(..) => match struct_fields(&self.cx.adts, t) {
                Some(fields) => {
                    let vals: Vec<Val> = fields
                        .iter()
                        .map(|(_, ft)| {
                            let ft = self.cx.solved(ft);
                            self.unknown(&ft)
                        })
                        .collect();
                    self.assume_fields(t, &vals);
                    Val::Struct(vals)
                }
                None => Val::Other,
            },
            _ => Val::Other,
        }
    }

    /// The refinements of the fields of `t`, if it's a struct that has any.
    fn refined_fields(&self, t: &Type) -> Option<&RefinedFields> {
        match t {
            Type::Adt(symbol, ..) => self.cx.field_refined.get(symbol),
            _ => None,
        }
    }

    /// What's known of a value of struct `t` whose fields are `vals`: its
    /// refinements, which every value of it has.
    fn assume_fields(&mut self, t: &Type, vals: &[Val]) {
        let Some(fields) = self.refined_fields(t).cloned() else {
            return;
        };
        for (j, r) in fields.refs.iter().enumerate() {
            if let Some(r) = r
                && let Some(f) = self.pred(r, &vals[j], Env::Fields(vals))
            {
                self.fact(f);
            }
        }
    }

    fn ty_of(&self, e: &Expr) -> Type {
        match self.cx.typed.exprs.get(&e.id) {
            Some(t) => self.cx.solved(t),
            None => Type::Unit,
        }
    }

    fn ty_of_bind(&self, id: NodeId) -> Type {
        match self.cx.typed.binds.get(&id) {
            Some(b) => self.cx.solved(&b.ty),
            None => Type::Unit,
        }
    }

    /// The type of what's at place `p`.
    fn place_ty(&self, p: &Place) -> Type {
        let mut t = self.ty_of_bind(p.root);
        for proj in &p.path {
            t = match (proj, t) {
                (Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => self.cx.solved(&inner),
                (Proj::Field(i), t) => match struct_fields(&self.cx.adts, &t) {
                    Some(fields) if *i < fields.len() => self.cx.solved(&fields[*i].1),
                    _ => Type::Unit,
                },
                _ => Type::Unit,
            };
        }
        t
    }

    /// The place expression `e` is, if it's one.
    fn place_of(&self, e: &Expr) -> Option<Place> {
        match &e.kind {
            ExprKind::Var(..) => match self.cx.typed.vars.get(&e.id) {
                Some(Res::Local(id)) => Some(Place::local(*id)),
                _ => None,
            },
            ExprKind::Field(x, _) => {
                let (derefs, i) = *self.cx.typed.fields.get(&e.id)?;
                let mut p = self.place_of(x)?;
                for _ in 0..derefs {
                    p = self.deref(p);
                }
                Some(p.then(Proj::Field(i)))
            }
            ExprKind::Unary(UnOp::Deref, x) => Some(self.deref(self.place_of(x)?)),
            ExprKind::Move(x) => self.place_of(x),
            _ => None,
        }
    }

    /// What `p` points to: if `p` is a local bound to a `&` of a place,
    /// that place.
    fn deref(&self, p: Place) -> Place {
        if p.path.is_empty()
            && let Some(q) = self.alias.get(&p.root)
        {
            return q.clone();
        }
        p.then(Proj::Deref)
    }

    /// May what's at `p` change other than by assignment?
    fn is_unstable(&self, p: &Place) -> bool {
        (0..=p.path.len()).any(|n| {
            self.unstable.contains(&Place {
                root: p.root,
                path: p.path[..n].to_vec(),
            })
        })
    }

    fn fact(&mut self, f: Formula) {
        if f != Formula::True {
            self.facts.push(f);
        }
    }

    /// Does `goal` follow from what's known? Anything does where nothing
    /// gets to.
    fn proves(&self, goal: Option<Formula>) -> bool {
        if !self.reachable {
            return true;
        }
        match goal {
            Some(g) => logic::valid(&self.facts, &g, &self.kinds),
            None => false,
        }
    }

    /// Does pattern `pat` bind anything by `&mut`?
    fn binds_mut(&self, pat: &Pattern) -> bool {
        let mut any = false;
        each_bind(pat, &mut |p| {
            let id = self.cx.typed.sites.get(&p.id).copied().unwrap_or(p.id);
            any |= self.cx.typed.binds.get(&id).and_then(|b| b.by_ref) == Some(true);
        });
        any
    }

    /// Finds the places in `e` that may change other than by assignment:
    /// those borrowed `&mut`, by `&mut`, a method's `&mut self` or a
    /// pattern; what a `&mut` used whole points to, as whatever it's given
    /// to can write through it; and the locals a closure captures that are
    /// `mut` or hold a `&mut`. `projected` says `e` is only looked inside
    /// of, by a field, a `*`, a `&` or an assignment.
    fn unstable_in(&mut self, e: &Expr, projected: bool) {
        if !projected
            && let Type::Ref(_, true, _) = self.ty_of(e)
            && let Some(p) = self.place_of(e)
            && self.cx.typed.coerced.get(&e.id).is_none_or(|c| c.1)
        {
            let p = p.then(Proj::Deref);
            self.unstable.insert(p);
        }
        match &e.kind {
            ExprKind::Unary(UnOp::RefMut, x) => {
                if let Some(p) = self.place_of(x) {
                    self.unstable.insert(p);
                }
                self.unstable_in(x, true);
            }
            ExprKind::Unary(UnOp::Ref | UnOp::Deref, x) | ExprKind::Field(x, _) => {
                self.unstable_in(x, true);
            }
            ExprKind::Assign(_, place, value) => {
                self.unstable_in(place, true);
                self.unstable_in(value, false);
            }
            ExprKind::Call(callee, args) => {
                let named = self.fn_named(callee).is_some();
                let passed: Vec<Passed> = args.iter().map(|a| self.passed(a)).collect();
                self.borrows(e, named, &passed);
                self.unstable_in(callee, false);
                for (a, p) in args.iter().zip(&passed) {
                    match (&a.kind, &p.borrowed) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::MethodCall(x, _, _, args) => {
                let (named, receiver) = match self.cx.typed.methods.get(&e.id) {
                    Some((res, derefs, by)) => {
                        (matches!(res, Res::Fn(..)), self.receiver(x, *derefs, *by))
                    }
                    None => (false, self.passed(x)),
                };
                let mut passed = vec![receiver];
                passed.extend(args.iter().map(|a| self.passed(a)));
                self.borrows(e, named, &passed);
                self.unstable_in(x, true);
                for (a, p) in args.iter().zip(&passed[1..]) {
                    match (&a.kind, &p.borrowed) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                if arms.iter().any(|a| self.binds_mut(&a.pat))
                    && let Some(p) = self.place_of(scrutinee)
                {
                    self.unstable.insert(p);
                }
                self.unstable_in(scrutinee, false);
                for a in arms {
                    self.unstable_in(&a.body, false);
                }
            }
            ExprKind::Block(stmts, tail) => {
                for s in stmts {
                    match s {
                        Stmt::Let { pat, init, .. } => {
                            if let Some(init) = init {
                                if self.binds_mut(pat)
                                    && let Some(p) = self.place_of(init)
                                {
                                    self.unstable.insert(p);
                                }
                                self.unstable_in(init, false);
                            }
                        }
                        Stmt::Expr(e) | Stmt::BlockLike(e) => self.unstable_in(e, false),
                        Stmt::Fn(_) | Stmt::Type(_) => {}
                    }
                }
                if let Some(t) = tail {
                    self.unstable_in(t, false);
                }
            }
            ExprKind::Lambda(l) => {
                for id in self.cx.typed.captures.get(&l.id).into_iter().flatten() {
                    let Some(b) = self.cx.typed.binds.get(id) else {
                        continue;
                    };
                    let ty = self.cx.solved(&b.ty);
                    if b.mutable || self.has_mut_ref(&ty, &mut HashSet::new()) {
                        self.unstable.insert(Place::local(*id));
                    }
                }
                self.unstable_in(&l.body, false);
            }
            _ => {
                for c in e.children() {
                    self.unstable_in(c, false);
                }
            }
        }
    }

    /// What argument `a` passes: the place it lends `&mut`, if it does,
    /// by `&mut` or by passing on a `&mut` it is, and the type it's passed
    /// as.
    fn passed(&self, a: &Expr) -> Passed {
        let ty = self.ty_of(a);
        let shared = self.cx.typed.coerced.get(&a.id).is_some_and(|c| !c.1);
        let borrowed = match (&a.kind, &ty) {
            (ExprKind::Unary(UnOp::RefMut, x), _) => self.place_of(x),
            (_, Type::Ref(_, true, _)) if !shared => self.place_of(a).map(|p| p.then(Proj::Deref)),
            _ => None,
        };
        Passed {
            borrowed,
            ty,
            shared,
        }
    }

    /// What receiver `x` passes, got through `derefs` `*`s and then maybe
    /// a `&` or `&mut` (`by`).
    fn receiver(&self, x: &Expr, derefs: usize, by: Option<bool>) -> Passed {
        let mut ty = self.ty_of(x);
        for _ in 0..derefs {
            if let Type::Ref(_, _, inner) | Type::Box(inner) = ty {
                ty = self.cx.solved(&inner);
            }
        }
        let mut place = self.place_of(x);
        for _ in 0..derefs {
            place = place.map(|p| p.then(Proj::Deref));
        }
        let borrowed = match by {
            Some(true) => place,
            Some(false) => None,
            None => match ty {
                Type::Ref(_, true, _) => place.map(|p| p.then(Proj::Deref)),
                _ => None,
            },
        };
        let ty = match by {
            Some(m) => Type::Ref(Region::Erased, m, Box::new(ty)),
            None => ty,
        };
        Passed {
            borrowed,
            ty,
            shared: by == Some(false),
        }
    }

    /// The places call `e` lends `&mut`, as `passed` says, which `named`
    /// says calls a function by name. They're lent only while it runs if
    /// nothing can keep a reference after it: its result can't hold one,
    /// nor what its other arguments can write to, nor what's lent; and
    /// a function by name, unlike a closure, keeps nothing between calls.
    /// Then what's known of them is forgotten when it returns; or else they
    /// may change any time, as far as anything knows.
    fn borrows(&mut self, e: &Expr, named: bool, passed: &[Passed]) {
        let lent: Vec<Place> = passed.iter().filter_map(|p| p.borrowed.clone()).collect();
        if lent.is_empty() {
            return;
        }
        let seen = &mut HashSet::new();
        let short = named
            && !self.can_hold(&self.ty_of(e), seen)
            && passed.iter().all(|p| match &p.borrowed {
                Some(q) => !self.can_hold(&self.place_ty(q), seen),
                None if p.shared => true,
                None => match &p.ty {
                    Type::Ref(_, false, _) => true,
                    Type::Ref(_, true, inner) => !self.can_hold(&self.cx.solved(inner), seen),
                    t => !self.can_hold(t, seen),
                },
            });
        if short {
            self.short.insert(e.id, lent);
        } else {
            self.unstable.extend(lent);
        }
    }

    /// Can a value of type `t` hold a reference, or might it?
    fn can_hold(&self, t: &Type, seen: &mut HashSet<Type>) -> bool {
        match t {
            Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Fn(_) => false,
            Type::Ref(..) => true,
            Type::Box(inner) => self.can_hold(&self.cx.solved(inner), seen),
            Type::Adt(..) => {
                if !seen.insert(t.clone()) {
                    return false;
                }
                all_fields(&self.cx.adts, t)
                    .iter()
                    .any(|f| self.can_hold(&self.cx.solved(f), seen))
            }
            _ => true,
        }
    }

    /// Can something be written through a value of type `t`: does it hold
    /// a `&mut`, or might it?
    fn has_mut_ref(&self, t: &Type, seen: &mut HashSet<Type>) -> bool {
        match t {
            Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Fn(_) => false,
            Type::Ref(_, m, _) => *m,
            Type::Box(inner) => self.has_mut_ref(&self.cx.solved(inner), seen),
            Type::Adt(..) => {
                if !seen.insert(t.clone()) {
                    return false;
                }
                all_fields(&self.cx.adts, t)
                    .iter()
                    .any(|f| self.has_mut_ref(&self.cx.solved(f), seen))
            }
            _ => true,
        }
    }

    /// The value at place `p`.
    fn read_place(&mut self, p: &Place) -> Val {
        let t = self.place_ty(p);
        match t {
            Type::Int | Type::Bool => self.leaf(p, &t),
            Type::Adt(..) => match self.fields_of(p, &t) {
                Some(vals) => Val::Struct(vals),
                None => Val::Other,
            },
            _ => Val::Other,
        }
    }

    /// The values of the fields of the struct at `q`, of type `t`: each
    /// one's variable, and a new one for each nothing's known of, which,
    /// with the rest, has the struct's refinements.
    fn fields_of(&mut self, q: &Place, t: &Type) -> Option<Vec<Val>> {
        let fields = struct_fields(&self.cx.adts, t)?;
        let mut fresh = false;
        let mut vals = Vec::new();
        for (j, (_, ft)) in fields.iter().enumerate() {
            let ft = self.cx.solved(ft);
            let p = q.then(Proj::Field(j));
            let kind = match ft {
                Type::Int => Kind::Int,
                Type::Bool => Kind::Bool,
                _ => {
                    vals.push(self.read_place(&p));
                    continue;
                }
            };
            let stable = !self.is_unstable(&p);
            let v = match self.cur.get(&p) {
                Some(&v) if stable => v,
                _ => {
                    fresh = true;
                    let v = self.fresh(kind);
                    if stable {
                        self.cur.insert(p, v);
                    }
                    v
                }
            };
            vals.push(self.var(v));
        }
        if fresh {
            self.assume_fields(t, &vals);
        }
        Some(vals)
    }

    /// The `int` or `bool`, of type `t`, at place `p`.
    fn leaf(&mut self, p: &Place, t: &Type) -> Val {
        if let Some((q, i)) = p.field_of() {
            let qt = self.place_ty(&q);
            if let Some(mut vals) = self.fields_of(&q, &qt) {
                return vals.swap_remove(i);
            }
        }
        let stable = !self.is_unstable(p);
        if stable && let Some(&v) = self.cur.get(p) {
            return self.var(v);
        }
        let v = self.unknown(t);
        if let (true, Val::Int(_) | Val::Bool(_)) = (stable, &v) {
            let var = self.kinds.len() as Var - 1;
            self.cur.insert(p.clone(), var);
        }
        if p.path.is_empty() {
            self.assume_invariant(p.root, &v);
        }
        v
    }

    /// Forgets what's known of what's at `p`, and in it: it has a new value.
    fn kill(&mut self, p: &Place) {
        self.cur.retain(|q, _| !q.within(p));
    }

    /// Place `p` has a new value, `val` if it's known.
    fn write(&mut self, p: &Place, val: Option<Val>) {
        self.kill(p);
        let t = self.place_ty(p);
        self.set(p, &t, val);
    }

    /// What's at `p`, of type `t`, which nothing's known of, is `val`, if
    /// that's known: each `int` or `bool` in it stands for a new variable of
    /// the logic, equal to its part of `val`, which has its local's
    /// refinement, if it has one.
    fn set(&mut self, p: &Place, t: &Type, val: Option<Val>) {
        let kind = match t {
            Type::Int => Kind::Int,
            Type::Bool => Kind::Bool,
            Type::Adt(..) => {
                if let (Some(Val::Struct(vals)), Some(fields)) =
                    (val, struct_fields(&self.cx.adts, t))
                {
                    for (j, (v, (_, ft))) in vals.into_iter().zip(fields).enumerate() {
                        let ft = self.cx.solved(&ft);
                        self.set(&p.then(Proj::Field(j)), &ft, Some(v));
                    }
                }
                return;
            }
            _ => return,
        };
        let v = self.fresh(kind);
        if !self.is_unstable(p) {
            self.cur.insert(p.clone(), v);
        }
        if let Some(val) = val {
            for f in equal(&self.var(v), &val) {
                self.fact(f);
            }
        }
        if p.path.is_empty() {
            let now = self.var(v);
            self.assume_invariant(p.root, &now);
        }
    }

    /// Local `id` is bound, to `val` if it's known.
    fn bind(&mut self, id: NodeId, val: Option<Val>) {
        self.declared.push(id);
        self.write(&Place::local(id), val);
    }

    fn assume_invariant(&mut self, id: NodeId, val: &Val) {
        if let Some((r, _)) = self.invariants.get(&id).cloned()
            && let Some(f) = self.pred(&r, val, Env::Here)
        {
            self.fact(f);
        }
    }

    /// Refinement `r`'s predicate, of `val`, with its other names' values
    /// from `env`, if it can be said in the logic.
    fn pred(&mut self, r: &Refinement, val: &Val, env: Env) -> Option<Formula> {
        match self.term(&r.pred, val, env)? {
            Val::Bool(f) => Some(f),
            _ => None,
        }
    }

    fn term(&mut self, e: &Expr, binder: &Val, env: Env) -> Option<Val> {
        let int = |v: Val| match v {
            Val::Int(l) => Some(l),
            _ => None,
        };
        let boolean = |v: Val| match v {
            Val::Bool(f) => Some(f),
            _ => None,
        };
        Some(match &e.kind {
            ExprKind::Int(n) => Val::Int(Lin::constant(*n as i128)),
            ExprKind::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            ExprKind::Var(..) => match (self.cx.pred_names.get(&e.id)?, env) {
                (PredName::Binder, _) => binder.clone(),
                (PredName::Param(i), Env::Call(args)) => args.get(*i)?.clone(),
                (PredName::Param(i), Env::Here) => {
                    let id = *self.params.get(*i)?;
                    self.read_place(&Place::local(id))
                }
                (PredName::Local(id), _) => self.read_place(&Place::local(*id)),
                (PredName::Field(j), Env::Fields(vals)) => vals.get(*j)?.clone(),
                (PredName::Param(_), Env::Fields(_)) | (PredName::Field(_), _) => return None,
            },
            ExprKind::Unary(UnOp::Neg, x) => Val::Int(int(self.term(x, binder, env)?)?.scale(-1)?),
            ExprKind::Unary(UnOp::Not, x) => Val::Bool(boolean(self.term(x, binder, env)?)?.not()),
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.term(l, binder, env)?, self.term(r, binder, env)?);
                match (op, a, b) {
                    (BinOp::Add, Val::Int(a), Val::Int(b)) => Val::Int(a.add(&b)?),
                    (BinOp::Sub, Val::Int(a), Val::Int(b)) => Val::Int(a.sub(&b)?),
                    (BinOp::Mul, Val::Int(a), Val::Int(b)) => Val::Int(times(&a, &b)?),
                    (op, Val::Int(a), Val::Int(b)) => Val::Bool(compare(*op, &a, &b)?),
                    (BinOp::Eq, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b)),
                    (BinOp::Ne, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b).not()),
                    (BinOp::And, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::and([a, b])),
                    (BinOp::Or, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::or([a, b])),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// Fails with `what` at `span` unless refinement `r` holds of `val`,
    /// with names from `env`.
    fn need(
        &mut self,
        r: &Refinement,
        val: &Val,
        env: Env,
        span: Span,
        what: String,
    ) -> Result<(), Error> {
        let goal = self.pred(r, val, env);
        if self.proves(goal) {
            return Ok(());
        }
        Err(Error::ty(
            span,
            format!("can't prove `{}` for {what}", r.text()),
        ))
    }

    /// Fails at `span` unless the fields `vals` of a value of struct `t`
    /// have its refinements: all of them, or with `changed`, those about
    /// that field.
    fn need_fields(
        &mut self,
        t: &Type,
        vals: &[Val],
        changed: Option<usize>,
        span: Span,
    ) -> Result<(), Error> {
        let Some(fields) = self.refined_fields(t).cloned() else {
            return Ok(());
        };
        let which: Vec<usize> = match changed {
            Some(i) => fields.about(i).collect(),
            None => (0..fields.refs.len()).collect(),
        };
        for j in which {
            let Some(r) = &fields.refs[j] else {
                continue;
            };
            let mut what = format!("field `{}` of `{}`", fields.names[j], fields.name);
            if let Some(i) = changed
                && i != j
            {
                what += &format!(", which names `{}`", fields.names[i]);
            }
            self.need(r, &vals[j], Env::Fields(vals), span, what)?;
        }
        Ok(())
    }

    /// `v`, the function's result, at `span`, must have its refinement.
    fn result(&mut self, v: Val, span: Span) -> Result<(), Error> {
        if self.closures > 0 {
            return Ok(());
        }
        if let Some((r, name)) = self.ret.clone() {
            self.need(&r, &v, Env::Here, span, format!("the result of `{name}`"))?;
        }
        Ok(())
    }

    /// The state to run each branch from.
    fn snapshot(&self) -> Snap {
        Snap {
            facts: self.facts.len(),
            cur: self.cur.clone(),
            reachable: self.reachable,
            declared: self.declared.len(),
        }
    }

    /// Goes back to `start`.
    fn restore(&mut self, start: &Snap) {
        self.facts.truncate(start.facts);
        self.cur = start.cur.clone();
        self.reachable = start.reachable;
        self.declared.truncate(start.declared);
    }

    /// Runs `go` from `start`, assuming `assume`, and gives how it ended.
    fn branch(
        &mut self,
        start: &Snap,
        assume: Option<Formula>,
        go: impl FnOnce(&mut Self) -> Result<Val, Error>,
    ) -> Result<Outcome, Error> {
        self.restore(start);
        if let Some(f) = assume {
            self.fact(f);
        }
        let val = go(self)?;
        Ok(Outcome {
            facts: self.facts.split_off(start.facts),
            cur: std::mem::take(&mut self.cur),
            reachable: self.reachable,
            val,
        })
    }

    /// Where branches `outcomes`, all from `start`, meet: what's known is
    /// that one of them was taken, and each place is whichever value the
    /// branch taken gave it. A place a branch gave a new value nothing's
    /// known of, or never read, is nothing known in that one. Gives the
    /// value, of type `ty`.
    fn join(&mut self, start: &Snap, outcomes: Vec<Outcome>, ty: &Type) -> Val {
        self.restore(start);
        let reached: Vec<Outcome> = outcomes.into_iter().filter(|o| o.reachable).collect();
        if reached.is_empty() {
            self.reachable = false;
            return self.unknown(ty);
        }
        let in_scope: HashSet<NodeId> = self.declared.iter().copied().collect();
        if let [_] = &reached[..] {
            let o = reached.into_iter().next().expect("one");
            self.facts.extend(o.facts);
            self.cur = o.cur;
            self.cur.retain(|p, _| in_scope.contains(&p.root));
            return o.val;
        }
        let mut places: Vec<&Place> = (reached.iter())
            .flat_map(|o| o.cur.keys())
            .filter(|p| in_scope.contains(&p.root))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        places.sort();
        let mut cur = HashMap::new();
        let mut new: Vec<(Var, Vec<Option<Var>>)> = Vec::new();
        for p in places {
            let vars: Vec<Option<Var>> = reached.iter().map(|o| o.cur.get(p).copied()).collect();
            match vars[0] {
                Some(v) if vars.iter().all(|w| *w == Some(v)) => {
                    cur.insert(p.clone(), v);
                }
                _ => {
                    let had = vars.iter().flatten().next().expect("in one");
                    let n = self.fresh(self.kinds[*had as usize]);
                    cur.insert(p.clone(), n);
                    new.push((n, vars));
                }
            }
        }
        let result = self.unknown(ty);
        let mut cases = Vec::new();
        for (i, o) in reached.into_iter().enumerate() {
            let mut parts = o.facts;
            parts.extend(equal(&result, &o.val));
            for (n, vars) in &new {
                if let Some(v) = vars[i] {
                    parts.extend(equal(&self.var(*n), &self.var(v)));
                }
            }
            cases.push(Formula::and(parts));
        }
        self.fact(Formula::or(cases));
        self.cur = cur;
        result
    }

    /// Call `e` has returned: what it was lent `&mut` may have changed.
    fn returned(&mut self, e: &Expr) {
        for p in self.short.get(&e.id).cloned().into_iter().flatten() {
            self.kill(&p);
        }
    }

    /// The places assigned anywhere in `e`, or lent `&mut` to a call.
    fn assigned(&self, e: &Expr, out: &mut Vec<Place>) {
        if let ExprKind::Assign(_, place, _) = &e.kind
            && let Some(p) = self.place_of(place)
        {
            out.push(p);
        }
        out.extend(self.short.get(&e.id).into_iter().flatten().cloned());
        for c in e.children() {
            self.assigned(c, out);
        }
    }

    /// Each place in `places` has a value nothing's known of but its
    /// refinements.
    fn forget(&mut self, places: &[Place]) {
        for p in places {
            self.kill(p);
        }
    }

    /// `l op r`, an operator at node `site`: proves it can't panic if it
    /// can, and gives its value, which it has if it didn't panic.
    fn arith(&mut self, site: NodeId, op: BinOp, l: Val, r: Val) -> Val {
        let (Val::Int(a), Val::Int(b)) = (l, r) else {
            return Val::Int(Lin::var(self.fresh(Kind::Int)));
        };
        let (min, max) = (
            Lin::constant(i64::MIN as i128),
            Lin::constant(i64::MAX as i128),
        );
        let in_range = |l: &Lin| Some(Formula::and([Formula::le(&min, l)?, Formula::le(l, &max)?]));
        let (value, goal) = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let value = match op {
                    BinOp::Add => a.add(&b),
                    BinOp::Sub => a.sub(&b),
                    _ => times(&a, &b),
                };
                let goal = value.as_ref().and_then(in_range);
                (value, goal)
            }
            // Not by zero, nor the least `int` by -1.
            _ => {
                let goal = (|| {
                    let zero = Formula::eq(&b, &Lin::constant(0))?;
                    let overflow = Formula::and([
                        Formula::eq(&a, &min)?,
                        Formula::eq(&b, &Lin::constant(-1))?,
                    ]);
                    Some(Formula::and([zero.not(), overflow.not()]))
                })();
                (None, goal)
            }
        };
        if self.proves(goal) {
            self.proved.insert(site);
        }
        match value {
            Some(v) => Val::Int(v),
            None => Val::Int(Lin::var(self.fresh(Kind::Int))),
        }
    }

    /// What function `callee` names, if it names one: its symbol.
    fn fn_named(&self, callee: &Expr) -> Option<&str> {
        match self.cx.typed.vars.get(&callee.id) {
            Some(Res::Fn(symbol, _)) => Some(symbol),
            _ => None,
        }
    }

    /// Fails if `e`, which names function `symbol` but doesn't call it,
    /// names one with refined parameters.
    fn as_value(&self, symbol: &str, span: Span) -> Result<(), Error> {
        let Some(sig) = self.cx.refined.get(symbol) else {
            return Ok(());
        };
        if let Some(i) = sig.params.iter().position(Option::is_some) {
            return Err(Error::ty(
                span,
                format!(
                    "`{}` can't be used as a value: its parameter `{}` has a refinement, which \
                     a call through a value wouldn't check",
                    sig.name, sig.names[i]
                ),
            ));
        }
        Ok(())
    }

    /// A call at `e` of function `symbol`, if it's one, with arguments
    /// `args` (the receiver first, for a method) whose values are `vals`:
    /// checks its refined parameters, and gives its result.
    fn call(
        &mut self,
        e: &Expr,
        symbol: Option<&str>,
        args: &[&Expr],
        vals: Vec<Val>,
    ) -> Result<Val, Error> {
        let ty = self.ty_of(e);
        let sig = symbol.and_then(|s| self.cx.refined.get(s)).cloned();
        if let Some(sig) = &sig {
            for (i, r) in sig.params.iter().enumerate() {
                if let (Some(r), Some(arg)) = (r, args.get(i)) {
                    let what = format!("argument `{}` of `{}`", sig.names[i], sig.name);
                    self.need(r, &vals[i], Env::Call(&vals), arg.span, what)?;
                }
            }
        }
        if ty == Type::Never {
            self.reachable = false;
        }
        let result = self.unknown(&ty);
        if let Some(r) = sig.as_ref().and_then(|s| s.ret.as_ref())
            && let Some(f) = self.pred(r, &result, Env::Call(&vals))
        {
            self.fact(f);
        }
        Ok(result)
    }

    /// Fails if `x`, borrowed `&mut` at `span`, is a refined local, or a
    /// field a refinement is about.
    fn no_mut_borrow(&self, x: &Expr, span: Span) -> Result<(), Error> {
        if let (ExprKind::Var(name, _), Some(Res::Local(id))) =
            (&x.unmoved().kind, self.cx.typed.vars.get(&x.id))
            && self.invariants.contains_key(id)
        {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{name}` as `&mut`: its type has a refinement, which a write \
                     through the reference could break"
                ),
            ));
        }
        if let Some(why) = self.constrained_field(x) {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{}` as `&mut`: {why}, which a write through the reference \
                     could break",
                    crate::pretty::expr_text(x)
                ),
            ));
        }
        Ok(())
    }

    /// If `x` is a field a refinement is about, why it can't be borrowed
    /// `&mut`.
    fn constrained_field(&self, x: &Expr) -> Option<String> {
        let ExprKind::Field(base, _) = &x.unmoved().kind else {
            return None;
        };
        let (derefs, i) = *self.cx.typed.fields.get(&x.id)?;
        let mut t = self.ty_of(base);
        for _ in 0..derefs {
            t = match t {
                Type::Ref(_, _, inner) | Type::Box(inner) => self.cx.solved(&inner),
                _ => return None,
            };
        }
        self.refined_fields(&t)?.constrained(i)
    }

    /// The condition `c` as a formula, and what it is if it isn't a `bool`.
    fn cond(&mut self, c: &Expr) -> Result<Formula, Error> {
        Ok(match self.expr(c)? {
            Val::Bool(f) => f,
            _ => Formula::holds(self.fresh(Kind::Bool)),
        })
    }

    fn expr(&mut self, e: &Expr) -> Result<Val, Error> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Val::Int(Lin::constant(*n as i128))),
            ExprKind::Bool(b) => Ok(Val::Bool(if *b { Formula::True } else { Formula::False })),
            ExprKind::Unit => Ok(Val::Other),
            ExprKind::Var(..) => match self.cx.typed.vars.get(&e.id) {
                Some(Res::Local(id)) => Ok(self.read_place(&Place::local(*id))),
                Some(Res::Fn(symbol, _)) => {
                    self.as_value(symbol, e.span)?;
                    Ok(Val::Other)
                }
                _ => Ok(Val::Other),
            },
            ExprKind::Unary(UnOp::Neg, x) => {
                let v = self.expr(x)?;
                let Val::Int(a) = v else {
                    return Ok(self.unknown(&Type::Int));
                };
                let min = Lin::constant(i64::MIN as i128);
                if self.proves(Formula::lt(&min, &a)) {
                    self.proved.insert(e.id);
                }
                Ok(match a.scale(-1) {
                    Some(l) => Val::Int(l),
                    None => self.unknown(&Type::Int),
                })
            }
            ExprKind::Unary(UnOp::Not, x) => Ok(match self.expr(x)? {
                Val::Bool(f) => Val::Bool(f.not()),
                _ => self.unknown(&Type::Bool),
            }),
            ExprKind::Unary(UnOp::Deref, x) => match self.place_of(e) {
                Some(p) => Ok(self.read_place(&p)),
                None => {
                    self.expr(x)?;
                    let ty = self.ty_of(e);
                    Ok(self.unknown(&ty))
                }
            },
            ExprKind::Unary(op, x) => {
                if *op == UnOp::RefMut {
                    self.no_mut_borrow(x, e.span)?;
                }
                if self.place_of(x).is_none() {
                    self.expr(x)?;
                }
                Ok(Val::Other)
            }
            ExprKind::Binary(op @ (BinOp::And | BinOp::Or), l, r) => {
                let c = self.cond(l)?;
                let start = self.snapshot();
                let and = *op == BinOp::And;
                let right = self.branch(
                    &start,
                    Some(if and { c.clone() } else { c.clone().not() }),
                    |w| w.cond(r).map(Val::Bool),
                )?;
                let short = Val::Bool(if and { Formula::False } else { Formula::True });
                let left =
                    self.branch(&start, Some(if and { c.not() } else { c }), |_| Ok(short))?;
                Ok(self.join(&start, vec![right, left], &Type::Bool))
            }
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                Ok(match (op, a, b) {
                    (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem, a, b) => {
                        self.arith(e.id, *op, a, b)
                    }
                    (_, Val::Int(a), Val::Int(b)) => match compare(*op, &a, &b) {
                        Some(f) => Val::Bool(f),
                        None => self.unknown(&Type::Bool),
                    },
                    (BinOp::Eq, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b)),
                    (BinOp::Ne, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b).not()),
                    _ => self.unknown(&Type::Bool),
                })
            }
            ExprKind::Call(callee, args) => {
                let symbol = self.fn_named(callee).map(str::to_string);
                if symbol.is_none() {
                    self.expr(callee)?;
                }
                let vals = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let args: Vec<&Expr> = args.iter().collect();
                let v = self.call(e, symbol.as_deref(), &args, vals);
                self.returned(e);
                v
            }
            ExprKind::MethodCall(x, _, _, args) => {
                let method = self.cx.typed.methods.get(&e.id);
                if let Some((_, 0, Some(true))) = method {
                    self.no_mut_borrow(x, x.span)?;
                }
                let symbol = match method {
                    Some((Res::Fn(s, _), ..)) => Some(s.clone()),
                    _ => None,
                };
                let mut vals = vec![self.expr(x)?];
                for a in args {
                    vals.push(self.expr(a)?);
                }
                let all: Vec<&Expr> = std::iter::once(&**x).chain(args).collect();
                let v = self.call(e, symbol.as_deref(), &all, vals);
                self.returned(e);
                v
            }
            ExprKind::Variant(_, _, _, args, parens) => {
                let symbol = self.fn_named(e).map(str::to_string);
                if let Some(s) = &symbol
                    && !parens
                {
                    self.as_value(s, e.span)?;
                }
                let vals = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let args: Vec<&Expr> = args.iter().collect();
                match &symbol {
                    Some(_) if *parens => self.call(e, symbol.as_deref(), &args, vals),
                    _ => {
                        let ty = self.ty_of(e);
                        Ok(self.unknown(&ty))
                    }
                }
            }
            ExprKind::If(c, then, els) => {
                let c = self.cond(c)?;
                let start = self.snapshot();
                let a = self.branch(&start, Some(c.clone()), |w| w.expr(then))?;
                let b = self.branch(&start, Some(c.not()), |w| match els {
                    Some(els) => w.expr(els),
                    None => Ok(Val::Other),
                })?;
                let ty = self.ty_of(e);
                Ok(self.join(&start, vec![a, b], &ty))
            }
            ExprKind::Block(stmts, tail) => {
                for s in stmts {
                    self.stmt(s)?;
                }
                match tail {
                    Some(t) => self.expr(t),
                    None => Ok(Val::Other),
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                let s = self.expr(scrutinee)?;
                let place = self.place_of(scrutinee);
                let start = self.snapshot();
                let mut before: Vec<Formula> = Vec::new();
                let mut outcomes = Vec::new();
                for arm in arms {
                    let this = pattern_formula(&arm.pat, &s);
                    let assume =
                        Formula::and(before.iter().cloned().map(Formula::not).chain(this.clone()));
                    let o = self.branch(&start, Some(assume), |w| {
                        w.bind_pattern(&arm.pat, Some(&s), place.clone(), None)?;
                        w.expr(&arm.body)
                    })?;
                    outcomes.push(o);
                    before.extend(this);
                }
                let ty = self.ty_of(e);
                Ok(self.join(&start, outcomes, &ty))
            }
            ExprKind::Lambda(l) => {
                let start = self.snapshot();
                let loops = std::mem::take(&mut self.loops);
                self.closures += 1;
                for p in &l.params {
                    self.bind(p.name.id, None);
                }
                let result = self.expr(&l.body);
                self.closures -= 1;
                self.loops = loops;
                self.restore(&start);
                result?;
                Ok(Val::Other)
            }
            ExprKind::Assign(op, place, value) => {
                let v = self.expr(value)?;
                let target = self.place_of(place);
                let old = |w: &mut Self| match &target {
                    Some(p) => w.read_place(p),
                    None => w.unknown(&Type::Int),
                };
                match (&place.kind, self.cx.typed.vars.get(&place.id)) {
                    (ExprKind::Var(name, _), Some(Res::Local(id))) => {
                        let id = *id;
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = old(self);
                                self.arith(e.id, *op, old, v)
                            }
                        };
                        if let Some((r, _)) = self.invariants.get(&id).cloned() {
                            let what = format!("`{name}`");
                            self.need(&r, &new, Env::Here, value.span, what)?;
                        }
                        self.write(&Place::local(id), Some(new));
                    }
                    (ExprKind::Field(base, _), _) => {
                        let (derefs, i) = self.cx.typed.fields[&place.id];
                        // The struct the field is of, and its fields' values.
                        let (t, mut vals) = match &target {
                            Some(p) => {
                                let (q, _) = p.field_of().expect("a field");
                                let t = self.place_ty(&q);
                                let vals = self.fields_of(&q, &t).unwrap_or_default();
                                (t, vals)
                            }
                            None => {
                                let v = self.expr(base)?;
                                let mut t = self.ty_of(base);
                                for _ in 0..derefs {
                                    if let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                                        t = self.cx.solved(&inner);
                                    }
                                }
                                let vals = match v {
                                    Val::Struct(vals) if derefs == 0 => vals,
                                    _ => match self.unknown(&t) {
                                        Val::Struct(vals) => vals,
                                        _ => Vec::new(),
                                    },
                                };
                                (t, vals)
                            }
                        };
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = vals.get(i).cloned().unwrap_or(Val::Other);
                                self.arith(e.id, *op, old, v)
                            }
                        };
                        if i < vals.len() {
                            vals[i] = new.clone();
                            self.need_fields(&t, &vals, Some(i), value.span)?;
                        }
                        if let Some(p) = &target {
                            self.write(p, Some(new));
                        }
                    }
                    _ => {
                        if target.is_none() {
                            self.expr(place)?;
                        }
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = old(self);
                                self.arith(e.id, *op, old, v)
                            }
                        };
                        if let Some(p) = &target {
                            self.write(p, Some(new));
                        }
                    }
                }
                Ok(Val::Other)
            }
            ExprKind::While(label, c, body) => {
                let mut changed = Vec::new();
                self.assigned(e, &mut changed);
                self.forget(&changed);
                let c = self.cond(c)?;
                let start = self.snapshot();
                self.loops.push(Loop {
                    label: label.as_ref().map(|l| l.name.name.clone()),
                    broke: false,
                });
                self.fact(c.clone());
                let body = self.expr(body);
                let broke = self.loops.pop().expect("pushed").broke;
                body?;
                self.restore(&start);
                if broke {
                    self.forget(&changed);
                } else {
                    self.fact(c.not());
                }
                Ok(Val::Other)
            }
            ExprKind::Loop(label, body) => {
                let mut changed = Vec::new();
                self.assigned(e, &mut changed);
                self.forget(&changed);
                let start = self.snapshot();
                self.loops.push(Loop {
                    label: label.as_ref().map(|l| l.name.name.clone()),
                    broke: false,
                });
                let body = self.expr(body);
                let broke = self.loops.pop().expect("pushed").broke;
                body?;
                self.restore(&start);
                self.reachable = start.reachable && broke;
                self.forget(&changed);
                let ty = self.ty_of(e);
                Ok(self.unknown(&ty))
            }
            ExprKind::Return(x) => {
                let v = match x {
                    Some(x) => self.expr(x)?,
                    None => Val::Other,
                };
                let span = x.as_ref().map_or(e.span, |x| x.span);
                self.result(v, span)?;
                self.reachable = false;
                Ok(Val::Other)
            }
            ExprKind::Break(label, x) => {
                if let Some(x) = x {
                    self.expr(x)?;
                }
                let target = match label {
                    Some(l) => self
                        .loops
                        .iter_mut()
                        .rev()
                        .find(|lp| lp.label.as_deref() == Some(l.name.as_str())),
                    None => self.loops.last_mut(),
                };
                if let Some(lp) = target {
                    lp.broke = true;
                }
                self.reachable = false;
                Ok(Val::Other)
            }
            ExprKind::Continue(_) => {
                self.reachable = false;
                Ok(Val::Other)
            }
            ExprKind::StructLit(_, _, inits) => {
                let ty = self.ty_of(e);
                let names = struct_fields(&self.cx.adts, &ty).unwrap_or_default();
                let mut vals = vec![Val::Other; names.len()];
                let mut spans = vec![e.span; names.len()];
                for init in inits {
                    let v = self.expr(&init.value)?;
                    if let Some(i) = names.iter().position(|(n, _)| *n == init.name.name) {
                        vals[i] = v;
                        spans[i] = init.value.span;
                    }
                }
                if let Some(fields) = self.refined_fields(&ty).cloned() {
                    for (j, r) in fields.refs.iter().enumerate() {
                        if let Some(r) = r {
                            let what = format!("field `{}` of `{}`", fields.names[j], fields.name);
                            self.need(r, &vals[j], Env::Fields(&vals), spans[j], what)?;
                        }
                    }
                }
                Ok(Val::Struct(vals))
            }
            ExprKind::Field(x, _) => {
                if let Some(p) = self.place_of(e) {
                    return Ok(self.read_place(&p));
                }
                let v = self.expr(x)?;
                let (derefs, i) = self.cx.typed.fields[&e.id];
                if let (0, Val::Struct(mut vals)) = (derefs, v)
                    && i < vals.len()
                {
                    return Ok(vals.swap_remove(i));
                }
                let mut t = self.ty_of(x);
                for _ in 0..derefs {
                    if let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                        t = self.cx.solved(&inner);
                    }
                }
                match self.unknown(&t) {
                    Val::Struct(mut vals) if i < vals.len() => Ok(vals.swap_remove(i)),
                    _ => {
                        let ty = self.ty_of(e);
                        Ok(self.unknown(&ty))
                    }
                }
            }
            ExprKind::Move(x) => self.expr(x),
            ExprKind::For(_) | ExprKind::Range(..) => {
                unreachable!("rewritten when paths are resolved")
            }
        }
    }

    /// Binds the variables of `pat`, which matched `s` if it's known, at
    /// `place` if it's one: a variable bound by value to an `int`, a
    /// `bool` or a struct is equal to what it's bound to, and one bound by
    /// `&` to a place points to it. `of` is the field of a struct `pat`
    /// matches, by the struct's type, if it matches one.
    fn bind_pattern(
        &mut self,
        pat: &Pattern,
        s: Option<&Val>,
        mut place: Option<Place>,
        of: Option<(&Type, usize)>,
    ) -> Result<(), Error> {
        let mut s = s.filter(|v| !matches!(v, Val::Other)).cloned();
        let peels = self.cx.typed.peels.get(&pat.id).copied().unwrap_or(0);
        if peels > 0 {
            s = None;
            place = place.map(|p| (0..peels).fold(p, |p, _| self.deref(p)));
        }
        match &pat.kind {
            PatternKind::Bind(name, _) => {
                let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                match self.cx.typed.binds.get(&id).and_then(|b| b.by_ref) {
                    None => {
                        let val = match (s, &place) {
                            (Some(v), _) => Some(v),
                            (None, Some(p)) => Some(self.read_place(p)),
                            (None, None) => None,
                        };
                        self.bind(id, val);
                    }
                    Some(m) => {
                        if m && let Some((t, i)) = of
                            && let Some(why) = self.refined_fields(t).and_then(|f| f.constrained(i))
                        {
                            return Err(Error::ty(
                                pat.span,
                                format!(
                                    "can't bind `{name}` by `&mut`: {why}, which a write through \
                                     the reference could break"
                                ),
                            ));
                        }
                        self.bind(id, None);
                        if let (false, Some(p)) = (m, place) {
                            self.alias.insert(id, p);
                        }
                    }
                }
            }
            PatternKind::Struct(_, _, fields, _) => {
                let t = match self.cx.typed.struct_pats.get(&pat.id) {
                    Some(t) => self.cx.solved(t),
                    None => Type::Unit,
                };
                let names = struct_fields(&self.cx.adts, &t).unwrap_or_default();
                let vals = match (s, &place) {
                    (Some(Val::Struct(vals)), _) => vals,
                    (_, Some(p)) => match self.read_place(p) {
                        Val::Struct(vals) => vals,
                        _ => Vec::new(),
                    },
                    _ => match self.unknown(&t) {
                        Val::Struct(vals) => vals,
                        _ => Vec::new(),
                    },
                };
                for f in fields {
                    let i = names.iter().position(|(n, _)| *n == f.name.name);
                    let val = i.and_then(|i| vals.get(i));
                    let at = i.and_then(|i| Some(place.as_ref()?.then(Proj::Field(i))));
                    self.bind_pattern(&f.pat, val, at, i.map(|i| (&t, i)))?;
                }
            }
            PatternKind::Variant(_, _, _, pats) => {
                for p in pats {
                    self.bind_pattern(p, None, None, None)?;
                }
            }
            PatternKind::Or(pats) => {
                for p in pats {
                    self.bind_pattern(p, None, None, of)?;
                }
            }
            PatternKind::Move(p) => self.bind_pattern(p, s.as_ref(), place, of)?,
            PatternKind::Wild
            | PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), Error> {
        match s {
            Stmt::Let { pat, init, .. } => {
                let v = match init {
                    Some(init) => Some(self.expr(init)?),
                    None => None,
                };
                if let Some(r) = self.cx.refined_lets.get(&pat.id).cloned() {
                    let name = pat.binds().pop().unwrap_or_default();
                    let v = v.clone().unwrap_or(Val::Other);
                    let span = init.as_ref().map_or(pat.span, |i| i.span);
                    self.need(&r, &v, Env::Here, span, format!("`{name}`"))?;
                    self.invariants.insert(pat.id, (r, name));
                }
                let place = init.as_ref().and_then(|i| self.place_of(i));
                match init {
                    Some(_) => self.bind_pattern(pat, v.as_ref(), place, None)?,
                    None => self.bind_pattern(pat, None, None, None)?,
                }
                // `let r = &p;`: `*r` is `p`, while `r` can be used.
                if let (Some(init), PatternKind::Bind(_, false)) = (init, &pat.kind)
                    && let ExprKind::Unary(UnOp::Ref, x) = &init.kind
                    && let Some(p) = self.place_of(x)
                {
                    let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                    self.alias.insert(id, p);
                }
                Ok(())
            }
            Stmt::Expr(e) | Stmt::BlockLike(e) => self.expr(e).map(drop),
            Stmt::Fn(_) | Stmt::Type(_) => Ok(()),
        }
    }
}

/// That `a` and `b` are equal, as far as the logic can say.
fn equal(a: &Val, b: &Val) -> Vec<Formula> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Formula::eq(a, b).into_iter().collect(),
        (Val::Bool(a), Val::Bool(b)) => vec![Formula::iff(a.clone(), b.clone())],
        (Val::Struct(a), Val::Struct(b)) => {
            a.iter().zip(b).flat_map(|(a, b)| equal(a, b)).collect()
        }
        _ => Vec::new(),
    }
}

/// `a * b`, if one of them is a constant.
fn times(a: &Lin, b: &Lin) -> Option<Lin> {
    match (a.as_constant(), b.as_constant()) {
        (Some(k), _) => b.scale(k),
        (_, Some(k)) => a.scale(k),
        _ => None,
    }
}

/// `a op b`, for a comparison `op`.
fn compare(op: BinOp, a: &Lin, b: &Lin) -> Option<Formula> {
    match op {
        BinOp::Lt => Formula::lt(a, b),
        BinOp::Le => Formula::le(a, b),
        BinOp::Gt => Formula::lt(b, a),
        BinOp::Ge => Formula::le(b, a),
        BinOp::Eq => Formula::eq(a, b),
        BinOp::Ne => Some(Formula::eq(a, b)?.not()),
        _ => None,
    }
}

/// That `pat` matches `s`, if that can be said: `None` for a pattern of a
/// struct or enum, which nothing's known of.
fn pattern_formula(pat: &Pattern, s: &Val) -> Option<Formula> {
    match (&pat.kind, s) {
        (PatternKind::Wild | PatternKind::Bind(..), _) => Some(Formula::True),
        (PatternKind::Int(n), Val::Int(l)) => Formula::eq(l, &Lin::constant(*n as i128)),
        (PatternKind::Range(lo, hi, _), Val::Int(l)) => Some(Formula::and([
            Formula::le(&Lin::constant(*lo as i128), l)?,
            Formula::le(l, &Lin::constant(*hi as i128))?,
        ])),
        (PatternKind::Bool(b), Val::Bool(f)) => Some(if *b { f.clone() } else { f.clone().not() }),
        (PatternKind::Or(alts), _) => Some(Formula::or(
            alts.iter()
                .map(|p| pattern_formula(p, s))
                .collect::<Option<Vec<_>>>()?,
        )),
        (PatternKind::Move(p), _) => pattern_formula(p, s),
        _ => None,
    }
}
