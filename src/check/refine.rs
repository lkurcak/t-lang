//! Refinement types: `{ d: i64 | d != 0 }`, the `i64`s that aren't 0 (see
//! docs/08-refinement-types.md and docs/08.1-field-refinements.md).
//!
//! A refinement is the whole type of a parameter, a result, a `let` or a
//! struct's field, never part of another type, and only of an `i64` or a
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
//! patterns of a `match` on an `i64`, from what a `let` or an assignment
//! gives a *place* (a variable, or a field or `*` of one: `self.start`),
//! and from operators that returned, as they'd have panicked otherwise.
//! What an operator gives is exact where it's linear (`+`, `-`, `!` of an
//! integer, `*`, `/`, `%`, `<<` and `>>` by a constant), and otherwise a
//! new value of which some lemmas say something (`bitwise`).
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

mod diagnostics;
mod logic;

use super::Checker;
use super::typed::Res;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::traits::{INT, SIGNED, UNSIGNED};
use crate::types::{IntTy, Type, all_fields, struct_fields};
use diagnostics::Invalidation;
pub(super) use diagnostics::ProofFailure;
use logic::{Failure, Formula, Kind, Lin, Var};
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

/// What a predicate's part is, in the logic: an integer, of ℤ, whatever
/// the integer types of what it names, or a `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sort {
    Int,
    Bool,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Sort::Int => "an integer",
            Sort::Bool => "`bool`",
        })
    }
}

/// The refinements in a function's signature, by symbol (`Checker::refined`).
#[derive(Debug, Clone)]
pub(super) struct RefinedSig {
    name: String,
    /// Its parameters' names.
    names: Vec<String>,
    /// Its type parameters, and its impl's, in order, as its type arguments
    /// are: what its const parameters are at a call.
    type_params: Vec<String>,
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

/// A measure: a method of `&self` marked `#[measure]`, which a predicate
/// can call (see docs/21-measures.md). What it gives depends on the value
/// alone, and it always gives something: so it's a function of the value,
/// in the logic as in the program.
#[derive(Debug, Clone)]
pub(super) struct Measure {
    name: String,
    /// What it gives: an integer type or `bool`.
    ret: Type,
    /// The module it's in, where it's its body; elsewhere it's only what's
    /// been learned of it.
    module: usize,
    /// Its body, a term of the logic over `self`'s fields and measures, or
    /// `None` for an intrinsic.
    body: Option<Expr>,
    /// Is it of a slice or a string, so of the reference to one?
    unsized_self: bool,
    /// Is it an array's intrinsic, its length?
    of_array: bool,
    /// The measures its body uses, once it's been checked.
    uses: Vec<String>,
    /// Does its body read a field? If not, it's only other measures, so
    /// it can be its body anywhere, without seeing what its module hides.
    reads_fields: bool,
}

/// A signature's or a struct's refinements, to be checked once every
/// impl is collected, as a predicate may call a measure of any.
#[derive(Debug, Clone)]
pub(super) enum Pending {
    Sig {
        f: Box<FnDecl>,
        params: Vec<Type>,
        ret: Type,
        allowed: bool,
        owner: (String, String),
        /// Its const parameters, and its impl's, as declared.
        consts: Vec<Ident>,
    },
    Fields {
        s: Box<StructDecl>,
        symbol: String,
    },
}

/// What a name in a predicate is.
#[derive(Debug, Clone)]
pub(super) enum PredName {
    /// The refinement's own, the value refined.
    Binder,
    /// A field of the struct, by index, in a field's refinement.
    Field(usize),
    /// The function's parameter, by index.
    Param(usize),
    /// A local variable, by id.
    Local(NodeId),
    /// A const parameter of the function, or of the struct, in a field's
    /// refinement, by name.
    Const(String),
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
        // `Self` is the impl's type, in an impl's method.
        let (params, ret) = match &self.sig_self {
            Some(t) => {
                let s = crate::types::Subst::new(&["Self".to_string()], std::slice::from_ref(t));
                (params.iter().map(|p| s.apply(p)).collect(), s.apply(ret))
            }
            None => (params.to_vec(), ret.clone()),
        };
        let (params, ret) = (&params[..], &ret);
        if f.measure.is_some() {
            self.measure_signature(f, params, ret)?;
        }
        let written = f.params.iter().map(|p| &p.ty).chain([&f.ret]);
        if written.clone().all(|t| t.refinement().is_none()) {
            return Ok(());
        }
        // Its predicates may call measures of impls not collected yet, so
        // they're checked once all are.
        let mut consts = self.const_decls.clone();
        consts.extend(
            f.generics
                .types
                .iter()
                .filter(|p| f.generics.is_const(p))
                .cloned(),
        );
        self.pending_refined.push(Pending::Sig {
            f: Box::new(f.clone()),
            params: params.to_vec(),
            ret: ret.clone(),
            allowed,
            owner: self.sig_owner.clone(),
            consts,
        });
        Ok(())
    }

    /// The parameters of function `symbol` its refinements name, by index.
    pub(super) fn named_params(&self, symbol: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(sig) = self.refined.get(symbol) else {
            return out;
        };
        for r in sig.params.iter().chain([&sig.ret]).flatten() {
            each_expr(&r.pred, &mut |e| {
                if let Some(PredName::Param(j)) = self.pred_names.get(&e.id) {
                    out.push(*j);
                }
            });
        }
        out
    }

    /// Checks the refinements whose checks waited for every impl to be
    /// collected (see `Pending`).
    pub(super) fn pending_refinements(&mut self) -> Result<(), Error> {
        for p in std::mem::take(&mut self.pending_refined) {
            match p {
                Pending::Sig {
                    f,
                    params,
                    ret,
                    allowed,
                    owner,
                    consts,
                } => {
                    let id = self.fns.iter().position(|d| d.symbol == owner.0);
                    self.pred_owner = id.map(super::scope::FnId);
                    self.sig_owner = owner;
                    self.module = f.module;
                    self.sig_refinements(&f, &params, &ret, allowed, &consts)?;
                }
                Pending::Fields { s, symbol } => {
                    self.pred_owner = None;
                    self.module = s.module;
                    self.fields_now(&s, &symbol)?;
                }
            }
        }
        Ok(())
    }

    fn sig_refinements(
        &mut self,
        f: &FnDecl,
        params: &[Type],
        ret: &Type,
        allowed: bool,
        consts: &[Ident],
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
        let type_params = match self.pred_owner {
            Some(id) => self.fns[id.0].scheme.params.to_vec(),
            None => Vec::new(),
        };
        let mut sig = RefinedSig {
            name: f.name.name.clone(),
            names: f.params.iter().map(|p| p.name.name.clone()).collect(),
            type_params,
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
                    .or_else(|| const_named(consts, name))
            };
            self.pred_again = p.mutable.map(|_| "a `mut` parameter");
            let r = self.refinement(&p.ty, &params[i], &scope);
            self.pred_again = None;
            sig.params.push(r?);
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
                .or_else(|| const_named(consts, name))
        };
        // What a result's refinement names is what the parameters were at
        // the call, so what's behind a `&mut` too.
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
        self.pending_refined.push(Pending::Fields {
            s: Box::new(s.clone()),
            symbol: symbol.to_string(),
        });
        Ok(())
    }

    fn fields_now(&mut self, s: &StructDecl, symbol: &str) -> Result<(), Error> {
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
        let consts: Vec<Ident> = (s.generics.types.iter())
            .filter(|p| s.generics.is_const(p))
            .cloned()
            .collect();
        for (i, f) in s.fields.iter().enumerate() {
            let level = |j: usize| s.fields[j].vis.level();
            let scope = |name: &str| {
                let Some(j) = s.fields.iter().position(|g| g.name.name == name) else {
                    return const_named(&consts, name);
                };
                let is = |l: Level| match l {
                    Level::Module => "is private",
                    Level::Package => "is `pub`",
                    Level::Export => "is exported",
                };
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
                } else if level(j) != level(i) {
                    Named::Not(format!(
                        "the refinement of `{}`, which {}, can't name `{name}`, which {}: a \
                         field's refinement can only name fields exactly as visible as it",
                        f.name.name,
                        is(level(i)),
                        is(level(j)),
                    ))
                } else {
                    Named::Value(types[j].clone(), PredName::Field(j), s.fields[j].name.span)
                })
            };
            self.pred_again = Some("a field");
            let r = self.refinement(&f.ty, &types[i], &scope);
            self.pred_again = None;
            let r = r?;
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
        if let PatternKind::Bind(_, true) = pat.kind {
            self.pred_again = Some("a `mut` variable");
        }
        self.pred_owner = self.current;
        let r = self.refinement(ann, t, &scope);
        self.pred_again = None;
        let r = r?.expect("refined");
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
    /// one, checked: its base is an `i64` or a `bool`, and its predicate a
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
        let kind = super::NoteKind::Var(binder.name.clone());
        self.note(binder.span, kind, Some(base.clone()), Some(binder.span));
        let scope = |name: &str| match name == binder.name {
            true => Some(Named::Value(base.clone(), PredName::Binder, binder.span)),
            false => scope(name),
        };
        let got = self.pred_type(pred, &scope)?;
        if got != Sort::Bool {
            return Err(Error::ty(
                pred.span,
                format!("expected `bool`, found {got}"),
            ));
        }
        Ok(Some(Refinement { pred: pred.clone() }))
    }

    /// The sort of predicate (part) `e`, which must be in the logic:
    /// linear arithmetic on integers, of any integer types, which are all
    /// the integers in it, ℤ, compared, and `bool`s.
    fn pred_type(
        &mut self,
        e: &Expr,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<Sort, Error> {
        let want = |cx: &mut Checker, e: &Expr, t: Sort| -> Result<(), Error> {
            let got = cx.pred_type(e, scope)?;
            if got != t {
                return Err(Error::ty(e.span, format!("expected {t}, found {got}")));
            }
            Ok(())
        };
        match &e.kind {
            ExprKind::Int(..) => Ok(Sort::Int),
            ExprKind::Bool(_) => Ok(Sort::Bool),
            ExprKind::Var(name, targs) if targs.is_empty() => match scope(name) {
                Some(Named::Value(t, what, def)) => {
                    let sort = match self.solved(&t) {
                        // An integer of a type not known yet.
                        _ if self.int_var(&t) => Sort::Int,
                        Type::Int(_) => Sort::Int,
                        Type::Bool => Sort::Bool,
                        t => {
                            return Err(Error::ty(
                                e.span,
                                format!(
                                    "a refinement can only name `{name}`, of type `{t}`, by its \
                                     measures: `{name}.m()`, where `m` is a `#[measure]`"
                                ),
                            ));
                        }
                    };
                    if let PredName::Const(c) = &what {
                        self.info.consts.insert(e.id, c.clone());
                    }
                    self.pred_names.insert(e.id, what.clone());
                    let kind = match what {
                        PredName::Field(_) => super::NoteKind::Field(name.clone()),
                        PredName::Const(_) => super::NoteKind::ConstParam(name.clone()),
                        _ => super::NoteKind::Var(name.clone()),
                    };
                    self.note(e.span, kind, Some(self.solved(&t)), Some(def));
                    Ok(sort)
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
            ExprKind::Unary(UnOp::Neg, x) => want(self, x, Sort::Int).map(|()| Sort::Int),
            ExprKind::Unary(UnOp::Not, x) => want(self, x, Sort::Bool).map(|()| Sort::Bool),
            ExprKind::Binary(op, l, r) => match op {
                BinOp::Add | BinOp::Sub => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    Ok(Sort::Int)
                }
                BinOp::Mul => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    if !constant(l) && !constant(r) {
                        return Err(Error::ty(
                            e.span,
                            format!(
                                "a refinement can only multiply by a constant: `{}` isn't linear",
                                crate::pretty::expr_text(e)
                            ),
                        ));
                    }
                    Ok(Sort::Int)
                }
                BinOp::Div
                | BinOp::Rem
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => Err(Error::ty(
                    e.span,
                    format!(
                        "a refinement can't use `{op}` yet: only `+`, `-` and `*` by a constant"
                    ),
                )),
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    Ok(Sort::Bool)
                }
                BinOp::Eq | BinOp::Ne => {
                    let t = self.pred_type(l, scope)?;
                    want(self, r, t)?;
                    Ok(Sort::Bool)
                }
                BinOp::And | BinOp::Or => {
                    want(self, l, Sort::Bool)?;
                    want(self, r, Sort::Bool)?;
                    Ok(Sort::Bool)
                }
            },
            ExprKind::MethodCall(x, m, targs, args) if targs.is_empty() && args.is_empty() => {
                let ExprKind::Var(name, targs) = &x.kind else {
                    return Err(Error::ty(
                        x.span,
                        "a refinement can only call a measure of a name: `s.len()`",
                    ));
                };
                let ty = match scope(name) {
                    Some(Named::Value(t, what, def)) if targs.is_empty() => {
                        self.pred_names.insert(x.id, what.clone());
                        let kind = match what {
                            PredName::Field(_) => super::NoteKind::Field(name.clone()),
                            PredName::Const(_) => super::NoteKind::ConstParam(name.clone()),
                            _ => super::NoteKind::Var(name.clone()),
                        };
                        self.note(x.span, kind, Some(self.solved(&t)), Some(def));
                        t
                    }
                    // A name that isn't a value: the error it is alone.
                    _ => {
                        self.pred_type(x, scope)?;
                        unreachable!("a name that isn't a value is an error")
                    }
                };
                let (symbol, derefs, through_mut) = self.find_measure(&ty, m)?;
                if through_mut && let Some(what) = self.pred_again {
                    return Err(Error::ty(
                        x.span,
                        format!(
                            "the refinement of {what} can't name a measure of what's behind \
                             `&mut` `{name}`, which can change"
                        ),
                    ));
                }
                let ret = self.measures[&symbol].ret.clone();
                if let Some(i) = self.fns.iter().position(|f| f.symbol == symbol) {
                    let (ty, def) = (self.fns[i].scheme.ty.clone(), self.fns[i].name.span);
                    let kind = super::NoteKind::Fn(m.name.clone());
                    self.note(m.span, kind, Some(ty), Some(def));
                    // What names it uses it, or else the struct does.
                    match self.pred_owner {
                        Some(owner) => {
                            self.uses[owner.0].fns.insert(super::scope::FnId(i));
                        }
                        None => self.kept_measures.push(super::scope::FnId(i)),
                    }
                }
                let call = (symbol.clone(), derefs, Some(false));
                self.info.measures.calls.insert(e.id, call);
                self.pred_measures.insert(e.id, (symbol, derefs));
                Ok(match ret {
                    Type::Bool => Sort::Bool,
                    _ => Sort::Int,
                })
            }
            _ => Err(Error::ty(
                e.span,
                "a refinement can only use literals, names, measures, `+`, `-`, `*` by a \
                 constant, comparisons, `&&`, `||` and `!`",
            )),
        }
    }

    /// The measure `m` of a value of type `ty`, or of what it points to,
    /// through however many `&`s, `&mut`s and boxes: its symbol, how many
    /// `*`s it's through, and whether one of them is a `&mut`'s, so that
    /// what it's of may change. A slice's or a string's measure is of the
    /// reference to it, which says how long it is, so the last `*` before
    /// one doesn't count: what's written through a `&mut [T]` can't change
    /// that.
    fn find_measure(&self, ty: &Type, m: &Ident) -> Result<(String, usize, bool), Error> {
        let mut t = self.solved(ty);
        let mut muts: Vec<bool> = Vec::new();
        loop {
            if let Some(symbol) = self.inherent_named(&t, &m.name) {
                let Some(measure) = self.measures.get(&symbol) else {
                    break;
                };
                let counted = match measure.unsized_self {
                    true => &muts[..muts.len().saturating_sub(1)],
                    false => &muts[..],
                };
                let through_mut = counted.iter().any(|m| *m);
                let fn_def = self.fns.iter().find(|f| f.symbol == symbol);
                if let Some(f) = fn_def
                    && !self.modules.visible(f.module, f.level, self.module)
                {
                    return Err(Error::ty(
                        m.span,
                        format!(
                            "measure `{}` of `{t}` {}",
                            m.name,
                            self.modules.hidden_by(f.module, f.level)
                        ),
                    ));
                }
                return Ok((symbol, muts.len(), through_mut));
            }
            t = match t {
                Type::Ref(_, mutable, inner) => {
                    muts.push(mutable);
                    self.solved(&inner)
                }
                Type::Box(inner) => {
                    muts.push(false);
                    self.solved(&inner)
                }
                _ => break,
            };
        }
        Err(Error::ty(
            m.span,
            format!(
                "`{}` isn't a measure: a refinement can only call methods marked `#[measure]`",
                m.name
            ),
        ))
    }

    /// Checks the signature of measure `f`, whose parameters and result
    /// have types `params` and `ret`, and keeps it.
    fn measure_signature(&mut self, f: &FnDecl, params: &[Type], ret: &Type) -> Result<(), Error> {
        let of = match (f.receiver(), params) {
            (Some(_), [Type::Ref(_, false, of)]) => self.solved(of),
            _ => {
                return Err(Error::ty(f.parens, "a measure takes only `&self`"));
            }
        };
        if !f.generics.is_empty() {
            return Err(Error::ty(
                f.name.span,
                "a measure has no type parameters of its own",
            ));
        }
        if let Some(e) = &f.effects {
            return Err(Error::ty(
                e.span,
                "a measure can't have effects: what it gives is all it does",
            ));
        }
        let ret = self.solved(ret);
        if !matches!(ret, Type::Int(_) | Type::Bool) {
            return Err(Error::ty(
                f.ret.span,
                format!("a measure gives an integer or a `bool`, not `{ret}`"),
            ));
        }
        let body = f.intrinsic.is_none().then(|| f.body.clone());
        let symbol = self.sig_owner.0.clone();
        self.info.measures.bodies.insert(symbol, body);
        let measure = Measure {
            name: f.name.name.clone(),
            ret,
            module: f.module,
            body: f.intrinsic.is_none().then(|| f.body.clone()),
            unsized_self: matches!(of, Type::Slice(_) | Type::Str),
            of_array: matches!(of, Type::Array(..)) && f.intrinsic.is_some(),
            uses: Vec::new(),
            reads_fields: true,
        };
        self.measures.insert(self.sig_owner.0.clone(), measure);
        Ok(())
    }

    /// Checks the body of measure `symbol`, `body`, is a term of the logic:
    /// `self`'s fields, measures, literals and what a predicate can do with
    /// them; and that it isn't defined by itself, through other measures.
    pub(super) fn measure_body(&mut self, symbol: &str, body: &Expr) -> Result<(), Error> {
        let mut uses = Vec::new();
        let term = match &body.kind {
            ExprKind::Block(stmts, Some(tail)) if stmts.is_empty() => &**tail,
            _ => body,
        };
        self.measure_term(term, false, &mut uses)?;
        uses.dedup();
        let mut reads_fields = false;
        each_expr(term, &mut |e| {
            reads_fields |= matches!(e.kind, ExprKind::Field(..))
        });
        let m = self.measures.get_mut(symbol).expect("a measure");
        m.uses = uses;
        m.reads_fields = reads_fields;
        // Measures are checked one at a time, so a cycle is found once the
        // last of it is.
        let mut path = vec![symbol.to_string()];
        if let Some(through) = self.measure_cycle(symbol, &mut path) {
            let name = |s: &str| self.measures[s].name.clone();
            return Err(Error::ty(
                body.span,
                format!(
                    "measure `{}` is defined by itself, through `{}`",
                    name(symbol),
                    name(&through)
                ),
            ));
        }
        Ok(())
    }

    /// A measure on the way from the last of `path` back to `start`, if
    /// there's a way.
    fn measure_cycle(&self, start: &str, path: &mut Vec<String>) -> Option<String> {
        let last = path.last().expect("not empty").clone();
        for u in &self.measures[&last].uses {
            if u == start {
                return Some(path.get(1).unwrap_or(&last).clone());
            }
            if path.contains(u) {
                continue;
            }
            path.push(u.clone());
            if let Some(t) = self.measure_cycle(start, path) {
                return Some(t);
            }
            path.pop();
        }
        None
    }

    /// Checks `e`, in a measure's body, is a term of the logic, keeping
    /// the measures it uses in `uses`. Only a `receiver` may be other than
    /// an integer or a `bool`: `self`, or a field of it, whose measure
    /// it's of.
    fn measure_term(
        &mut self,
        e: &Expr,
        receiver: bool,
        uses: &mut Vec<String>,
    ) -> Result<(), Error> {
        let bad = || {
            Error::ty(
                e.span,
                "a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*` \
                 by a constant, comparisons, `&&`, `||` and `!`",
            )
        };
        let ty = self.typed.exprs.get(&e.id).map(|t| self.solved(t));
        let scalar = matches!(ty, Some(Type::Int(_) | Type::Bool));
        if !receiver && !scalar {
            return Err(bad());
        }
        match &e.kind {
            ExprKind::Int(..) | ExprKind::Bool(_) => Ok(()),
            ExprKind::Var(name, _) if name == "self" && receiver => Ok(()),
            ExprKind::Field(x, _) => self.measure_term(x, true, uses),
            ExprKind::MethodCall(x, m, targs, args) if targs.is_empty() && args.is_empty() => {
                let symbol = match self.typed.methods.get(&e.id) {
                    Some((Res::Fn(s, _), ..)) if self.measures.contains_key(s) => s.clone(),
                    _ => {
                        return Err(Error::ty(
                            m.span,
                            format!(
                                "`{}` isn't a measure: a refinement can only call methods marked \
                                 `#[measure]`",
                                m.name
                            ),
                        ));
                    }
                };
                let (_, derefs, by) = self.typed.methods[&e.id].clone();
                self.info
                    .measures
                    .calls
                    .insert(e.id, (symbol.clone(), derefs, by));
                uses.push(symbol);
                self.measure_term(x, true, uses)
            }
            ExprKind::Unary(UnOp::Neg | UnOp::Not, x) => self.measure_term(x, false, uses),
            ExprKind::Binary(op, l, r) => {
                if matches!(
                    op,
                    BinOp::Div
                        | BinOp::Rem
                        | BinOp::BitAnd
                        | BinOp::BitOr
                        | BinOp::BitXor
                        | BinOp::Shl
                        | BinOp::Shr
                ) || (*op == BinOp::Mul && !constant(l) && !constant(r))
                    || self.is_ord_call(e.id)
                {
                    return Err(bad());
                }
                self.measure_term(l, false, uses)?;
                self.measure_term(r, false, uses)
            }
            _ => Err(bad()),
        }
    }

    /// Checks what the refinements in body `f` need, and works out which
    /// of its operators can't panic, taking those out of its effects.
    pub(super) fn refine(&mut self, f: &FnDecl) -> Result<(), Error> {
        let id = self.current.expect("in a function");
        let symbol = self.fns[id.0].symbol.clone();
        let sig = self.refined.get(&symbol).cloned();
        if self.measures.contains_key(&symbol) {
            self.measure_body(&symbol, &f.body)?;
        }
        let mut w = Walk {
            cx: self,
            kinds: Vec::new(),
            facts: Vec::new(),
            laws: Vec::new(),
            reachable: true,
            cur: HashMap::new(),
            objs: HashMap::new(),
            next_obj: 0,
            measured: HashMap::new(),
            module: self.fns[id.0].module,
            invariants: HashMap::new(),
            unstable: HashSet::new(),
            lent: HashMap::new(),
            alias: HashMap::new(),
            declared: Vec::new(),
            proved: HashSet::new(),
            params: f.params.iter().map(|p| p.name.id).collect(),
            entry: Vec::new(),
            ret: None,
            closures: 0,
            loops: Vec::new(),
            size: None,
            bounds: HashMap::new(),
            consts: HashMap::new(),
            failures: HashMap::new(),
            invalidated: Vec::new(),
        };
        w.unstable_in(&f.body, false);
        let mut ret = None;
        for (i, p) in f.params.iter().enumerate() {
            if let Some(r) = sig.as_ref().and_then(|s| s.params[i].clone()) {
                w.invariants.insert(p.name.id, (r, p.name.name.clone()));
            }
            w.bind(p.name.id, None);
        }
        // What the parameters are at the call, which a result's refinement
        // is about: what's behind a `&mut` too, which may change after.
        for p in &f.params {
            let v = w.read_place(&Place::local(p.name.id));
            let v = w.settled(v, ENTRY_DEPTH);
            w.entry.push(v);
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
        let failures = std::mem::take(&mut w.failures);
        for (node, failure) in failures {
            let span = self
                .effect_sites
                .iter()
                .filter(|s| s.node == Some(node))
                .map(|s| s.span)
                .chain(
                    self.withins
                        .iter()
                        .filter(|s| s.node == Some(node))
                        .map(|s| s.span),
                )
                .next();
            if let Some(span) = span {
                self.proof_failures.insert(span, failure);
            }
        }
        let keep = |node: Option<NodeId>| node.is_none_or(|n| !proved.contains(&n));
        self.effect_sites.retain(|s| keep(s.node));
        self.withins.retain(|w| keep(w.node));
        self.info.proved.extend(proved);
        Ok(())
    }
}

/// What `name` names in a predicate, if it's one of the const parameters
/// `consts`: a `usize`, the same all through the function or value.
fn const_named(consts: &[Ident], name: &str) -> Option<Named> {
    let p = consts.iter().rev().find(|p| p.name == name)?;
    let ty = Type::Int(crate::types::IntTy::Usize);
    Some(Named::Value(ty, PredName::Const(name.to_string()), p.span))
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
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) | PatternKind::Array(pats) => {
            pats.iter().for_each(|p| each_bind(p, f))
        }
        PatternKind::Struct(_, _, fields, _) => fields.iter().for_each(|fp| each_bind(&fp.pat, f)),
        PatternKind::Move(p) => each_bind(p, f),
        PatternKind::Wild
        | PatternKind::Int(..)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}

/// Is `e` a constant: no names in it?
fn constant(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(..) => true,
        ExprKind::Unary(UnOp::Neg, x) => constant(x),
        ExprKind::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul, l, r) => constant(l) && constant(r),
        _ => false,
    }
}

/// What an expression is, to the logic: an `i64`, as a linear sum, a
/// `bool`, as a formula, a struct, as its fields' values, a reference or a
/// box, as what it points to, or anything else, of which nothing is known.
/// But for those, each value is an *object*, which says which value it
/// is, so what a measure gives of it (`Walk::measured`).
#[derive(Debug, Clone)]
enum Val {
    Int(Lin),
    Bool(Formula),
    Struct(Obj, Vec<Val>),
    /// An array, known by its length, which its type says: what its
    /// values are isn't followed.
    Array(Lin),
    Ref(Obj, Pointee),
    Obj(Obj),
    Other,
}

/// Which value a value that isn't an integer or a `bool` is, by number.
/// Each stands for one value, which never changes: a place that may have
/// changed is another object.
type Obj = u32;

/// What a reference or a box points to: a place, which is read when it's
/// needed, a value, or what isn't known.
#[derive(Debug, Clone)]
enum Pointee {
    Place(Place),
    Val(Box<Val>),
    Unknown,
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

/// The state to run each branch from.
struct Snap {
    facts: usize,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    declared: usize,
    invalidated: Vec<Invalidation>,
}

/// How a path through a branch ended: what it learned, what each place
/// stood for at its end, whether it gets there, and its value.
struct Outcome {
    facts: Vec<Formula>,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    val: Val,
    invalidated: Vec<Invalidation>,
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
    /// Which variable of the logic each `i64` or `bool` place stands for
    /// here. One that isn't in it is a value nothing's known of yet, but
    /// its refinements.
    cur: HashMap<Place, Var>,
    /// Which object the value at each place that isn't an integer or a
    /// `bool` is here, as `cur` does for those.
    objs: HashMap<Place, Obj>,
    /// How many objects there are.
    next_obj: Obj,
    /// What each measure gives of each object, once it's been asked: by
    /// the object and the measure's symbol. An object is one value, so a
    /// measure of it is one value too.
    measured: HashMap<(Obj, String), Val>,
    /// The module of the function walked, where its measures are their
    /// bodies.
    module: usize,
    /// The refinement of each refined local, with its name.
    invariants: HashMap<NodeId, (Refinement, String)>,
    /// The places that may change other than by assignment, with all
    /// that's in them.
    unstable: HashSet<Place>,
    /// The places each expression lends `&mut`, by its node: a call, a
    /// `&mut`, or a `&mut` used whole. What's known of them is forgotten
    /// once it's evaluated.
    lent: HashMap<NodeId, Vec<Place>>,
    /// What each local bound to a `&` of a place points to: `*r` is that
    /// place, which can't change while `r` can be used.
    alias: HashMap<NodeId, Place>,
    /// The locals in scope, in the order they were bound.
    declared: Vec<NodeId>,
    /// The operators proved not to panic, by node.
    proved: HashSet<NodeId>,
    /// The ids of the function's parameters.
    params: Vec<NodeId>,
    /// Their values at the call (see `Walk::settled`).
    entry: Vec<Val>,
    /// The refinement of the function's result, with its name.
    ret: Option<(Refinement, String)>,
    /// How many closures deep this is.
    closures: usize,
    loops: Vec<Loop>,
    /// The variable for the greatest `isize`, once there's one: how big
    /// it is depends on the machine. The greatest `usize` is twice it, plus
    /// one, and the least `isize` minus it, minus one.
    size: Option<Var>,
    /// The least and greatest values of each type parameter `T: Int`, once
    /// there are some: it can be any integer type, so they're variables,
    /// or, of a `T: Signed`, the least is -1 minus the greatest, and of a
    /// `T: Unsigned`, 0.
    bounds: HashMap<String, (Lin, Lin)>,
    /// The value of each const parameter of the function, once there's one:
    /// a `usize` the caller picks, the same all through.
    consts: HashMap<String, Val>,
    /// What's true everywhere in the function, of the variables of
    /// `size` and `bounds`, which no path forgets, as those are made once,
    /// on whichever path first needs them.
    laws: Vec<Formula>,
    /// Explanations saved at the point each operator's proof failed.
    failures: HashMap<NodeId, ProofFailure>,
    /// Source events that changed a place's value along this path.
    invalidated: Vec<Invalidation>,
}

/// How many references deep what a function's parameters point to is kept
/// as it is at the call.
const ENTRY_DEPTH: usize = 4;

/// How a predicate's names other than its binder get their values: those
/// of the function being walked, a call's arguments, or the fields of a
/// struct.
#[derive(Clone, Copy)]
enum Env<'a> {
    Here,
    /// At a call, with the arguments' values, and what the callee's type
    /// parameters are there, for its const parameters. One that isn't
    /// among them is the function's own.
    Call(&'a [Val], &'a [(String, Type)]),
    /// Of a struct's value, with its fields' values, and its type
    /// parameters' arguments.
    Fields(&'a [Val], &'a [(String, Type)]),
}

impl Walk<'_> {
    fn fresh(&mut self, kind: Kind) -> Var {
        self.kinds.push(kind);
        (self.kinds.len() - 1) as Var
    }

    /// Variable `v` of the logic, as a value.
    fn var(&self, v: Var) -> Val {
        match self.kinds[v as usize] {
            Kind::Int(..) | Kind::Bound(..) => Val::Int(Lin::var(v)),
            Kind::Bool => Val::Bool(Formula::holds(v)),
        }
    }

    /// The least and greatest values of `t`, if it's an integer type, or a
    /// type parameter that's `Int`, which can be any of them. What's known
    /// of one whose size isn't, as the logic says, is only what holds of
    /// all it can be: of `isize` and `usize`, what holds of 32 and 64 bits,
    /// and of a type parameter, what holds of every integer type, that its
    /// least is at most 0 and its greatest at least 127.
    fn range(&mut self, t: &Type) -> Option<(Lin, Lin)> {
        let size = |w: &mut Self| match w.size {
            Some(v) => Lin::var(v),
            None => {
                let (_, lo) = IntTy::I32.range_64();
                let (_, hi) = IntTy::I64.range_64();
                let v = w.fresh(Kind::Bound(lo, hi));
                w.size = Some(v);
                Lin::var(v)
            }
        };
        match t {
            Type::Int(IntTy::Isize) => {
                let max = size(self);
                Some((max.scale(-1)?.plus(-1)?, max))
            }
            Type::Int(IntTy::Usize) => Some((Lin::constant(0), size(self).scale(2)?.plus(1)?)),
            Type::Int(t) => {
                let (lo, hi) = t.range()?;
                Some((Lin::constant(lo), Lin::constant(hi)))
            }
            // But for the surrogates, in between: see `scalar`.
            Type::Char => Some((Lin::constant(0), Lin::constant(0x10FFFF))),
            Type::Param(p) if self.is_int(t) => {
                if let Some(b) = self.bounds.get(p) {
                    return Some(b.clone());
                }
                let (min, _) = IntTy::I64.range_64();
                let (_, max) = IntTy::U64.range_64();
                let (every_lo, every_hi) = super::ints::param_range(self.cx, t);
                let b = if self.implements(t, UNSIGNED) {
                    let hi = self.fresh(Kind::Bound(every_hi, max));
                    (Lin::constant(0), Lin::var(hi))
                } else if self.implements(t, SIGNED) {
                    let (_, i64_max) = IntTy::I64.range_64();
                    let hi = Lin::var(self.fresh(Kind::Bound(every_hi, i64_max)));
                    (hi.scale(-1)?.plus(-1)?, hi)
                } else {
                    let (lo, hi) = (
                        Lin::var(self.fresh(Kind::Bound(min, every_lo))),
                        Lin::var(self.fresh(Kind::Bound(every_hi, max))),
                    );
                    // Two's complement: the greatest is at least -1 minus
                    // the least, as it's equal in a signed type, and the
                    // least is 0 in an unsigned one.
                    let law = Formula::le(&Lin::constant(-1), &hi.add(&lo)?)?;
                    self.laws.push(law);
                    (lo, hi)
                };
                self.bounds.insert(p.clone(), b.clone());
                Some(b)
            }
            _ => None,
        }
    }

    /// That `v`, a character's value, is a Unicode scalar value: in its
    /// range, and not a surrogate. Its range is a variable's kind, but the
    /// gap in it isn't, as a kind is an interval: it's said where a
    /// character's value turns into an integer, by `as`, the only place
    /// it's seen as a number.
    fn scalar(v: &Lin) -> Option<Formula> {
        Some(Formula::and([
            Formula::le(&Lin::constant(0), v)?,
            Formula::le(v, &Lin::constant(0x10FFFF))?,
            Formula::or([
                Formula::le(v, &Lin::constant(0xD7FF))?,
                Formula::le(&Lin::constant(0xE000), v)?,
            ]),
        ]))
    }

    /// Is `t` an integer type, a type parameter that's `Int`, or `char`:
    /// is its value a number, in the logic?
    fn is_scalar(&self, t: &Type) -> bool {
        matches!(t, Type::Char) || self.is_int(t)
    }

    /// Is `t` an integer type, or a type parameter that's `Int`?
    fn is_int(&self, t: &Type) -> bool {
        match t {
            Type::Int(_) => true,
            Type::Param(_) => self.implements(t, INT),
            _ => false,
        }
    }

    /// Does `t` implement built-in trait `name`, where the function is?
    fn implements(&self, t: &Type, name: &str) -> bool {
        let cx = self.cx;
        (cx.traits.implements(t, name, &cx.env, &cx.adts.proj.local)).is_ok()
    }

    /// The kind of variable for a value of type `t`, if it's an integer or
    /// a `bool`.
    fn kind(&mut self, t: &Type) -> Option<Kind> {
        match t {
            Type::Bool => Some(Kind::Bool),
            t => self.range(t).map(|(lo, hi)| Kind::Int(lo, hi)),
        }
    }

    /// A value of type `t` of which nothing is known: but if it's a
    /// struct, its fields have its refinements.
    fn unknown(&mut self, t: &Type) -> Val {
        match t {
            Type::Int(_) | Type::Param(_) | Type::Char if self.is_scalar(t) => match self.kind(t) {
                Some(k) => Val::Int(Lin::var(self.fresh(k))),
                None => Val::Other,
            },
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
                    Val::Struct(self.new_obj(), vals)
                }
                None => Val::Obj(self.new_obj()),
            },
            // What its type says of an array, its length, holds behind any
            // reference to it: nothing can change it.
            Type::Array(_, n) => match self.length(n) {
                Val::Int(l) => Val::Array(l),
                _ => Val::Obj(self.new_obj()),
            },
            Type::Ref(_, _, inner) | Type::Box(inner) if matches!(**inner, Type::Array(..)) => {
                let a = self.unknown(inner);
                Val::Ref(self.new_obj(), Pointee::Val(Box::new(a)))
            }
            Type::Ref(..) | Type::Box(_) => Val::Ref(self.new_obj(), Pointee::Unknown),
            t if has_obj(t) => Val::Obj(self.new_obj()),
            _ => Val::Other,
        }
    }

    fn new_obj(&mut self) -> Obj {
        self.next_obj += 1;
        self.next_obj - 1
    }

    /// The object the value at place `p` is: the one it was, if it can't
    /// have changed since, or a new one.
    /// Gives too whether it's a new one.
    fn obj_at(&mut self, p: &Place) -> (Obj, bool) {
        let stable = !self.is_unstable(p);
        if stable && let Some(&o) = self.objs.get(p) {
            return (o, false);
        }
        let o = self.new_obj();
        if stable {
            self.objs.insert(p.clone(), o);
        }
        (o, true)
    }

    /// What a reference or box `v` points to, if that's known.
    fn pointee(&mut self, v: Val) -> Val {
        match v {
            Val::Ref(_, Pointee::Place(q)) => self.read_place(&q),
            Val::Ref(_, Pointee::Val(v)) => *v,
            _ => Val::Other,
        }
    }

    /// The receiver `v` is, after `derefs` `*`s, and then a `&` or `&mut`
    /// if `by` says so: a reborrow of what a reference points to is that
    /// reference, the same object.
    fn autoref(&mut self, mut v: Val, derefs: usize, by: Option<bool>) -> Val {
        let peel = match by {
            Some(_) if derefs > 0 => derefs - 1,
            _ => derefs,
        };
        for _ in 0..peel {
            v = self.pointee(v);
        }
        match by {
            Some(_) if derefs == 0 => Val::Ref(self.new_obj(), Pointee::Val(Box::new(v))),
            _ => v,
        }
    }

    /// What measure `symbol` gives of the value `recv`, its `&self`, points
    /// to. In its module, a struct's is its body, of the struct's fields;
    /// elsewhere, and of anything else, it's a variable of the logic, the
    /// same each time it's asked of the same object. A slice's or a
    /// string's is of the reference, which says how long what it points to
    /// is.
    fn measure(&mut self, symbol: &str, recv: Val) -> Val {
        let m = self.cx.measures[symbol].clone();
        if let Some(body) = &m.body
            && (m.module == self.module || !m.reads_fields)
        {
            let term = match &body.kind {
                ExprKind::Block(_, Some(tail)) => &**tail,
                _ => body,
            };
            if let Some(v) = self.measure_term(term, &recv) {
                return v;
            }
        }
        // An array's is its length.
        if m.of_array
            && let Val::Array(l) = self.pointee(recv.clone())
        {
            return Val::Int(l);
        }
        let of = match (&recv, m.unsized_self) {
            (Val::Ref(o, _), true) => Some(*o),
            (_, true) => None,
            (_, false) => {
                let target = self.pointee(recv.clone());
                match target {
                    Val::Struct(o, _) | Val::Ref(o, _) | Val::Obj(o) => Some(o),
                    _ => None,
                }
            }
        };
        let Some(o) = of else {
            return self.unknown(&m.ret);
        };
        if let Some(v) = self.measured.get(&(o, symbol.to_string())) {
            return v.clone();
        }
        let v = self.unknown(&m.ret);
        self.measured.insert((o, symbol.to_string()), v.clone());
        v
    }

    /// The value of term `e` of a measure's body, whose `self` is `this`,
    /// if the logic can say it.
    fn measure_term(&mut self, e: &Expr, this: &Val) -> Option<Val> {
        Some(match &e.kind {
            ExprKind::Int(n, _) => Val::Int(Lin::constant(i128::from(*n))),
            ExprKind::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            ExprKind::Var(..) => this.clone(),
            ExprKind::Field(x, _) => {
                let (derefs, i) = *self.cx.typed.fields.get(&e.id)?;
                let mut v = self.measure_term(x, this)?;
                for _ in 0..derefs {
                    v = self.pointee(v);
                }
                match v {
                    Val::Struct(_, mut vals) if i < vals.len() => vals.swap_remove(i),
                    _ => return None,
                }
            }
            ExprKind::MethodCall(x, ..) => {
                let (Res::Fn(symbol, _), derefs, by) = self.cx.typed.methods.get(&e.id)?.clone()
                else {
                    return None;
                };
                let v = self.measure_term(x, this)?;
                let recv = self.autoref(v, derefs, by);
                self.measure(&symbol, recv)
            }
            ExprKind::Unary(UnOp::Neg, x) => match self.measure_term(x, this)? {
                Val::Int(a) => Val::Int(a.scale(-1)?),
                _ => return None,
            },
            ExprKind::Unary(UnOp::Not, x) => match self.measure_term(x, this)? {
                Val::Bool(f) => Val::Bool(f.not()),
                _ => return None,
            },
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.measure_term(l, this)?, self.measure_term(r, this)?);
                binary(*op, a, b)?
            }
            _ => return None,
        })
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
                && let Some(f) = self.pred(r, &vals[j], Env::Fields(vals, &self.adt_args(t)))
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
    fn proves(&self, goal: Option<&Formula>) -> Result<(), Failure> {
        if !self.reachable {
            return Ok(());
        }
        let Some(g) = goal else {
            return Err(Failure::Unsupported);
        };
        if self.laws.is_empty() {
            return logic::valid(&self.facts, g, &self.kinds);
        }
        let all: Vec<Formula> = self.facts.iter().chain(&self.laws).cloned().collect();
        logic::valid(&all, g, &self.kinds)
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

    /// Finds the places in `e` that may change other than by assignment.
    /// Those lent `&mut`, by `&mut` or to a call, and what a `&mut` used
    /// whole points to, as whatever it's given to can write through it,
    /// change only where they're lent and while the borrow lives, which
    /// the borrow checker says no one else can read them: so what's known
    /// of them is forgotten there (`Walk::lent`). Those borrowed `&mut` by a
    /// pattern, and the locals a closure captures that are `mut` or hold a
    /// `&mut`, may change any time, as far as anything here knows.
    /// `projected` says `e` is only looked inside of, by a field, a `*`, a
    /// `&` or an assignment.
    fn unstable_in(&mut self, e: &Expr, projected: bool) {
        if !projected
            && let Type::Ref(_, true, _) = self.ty_of(e)
            && let Some(p) = self.place_of(e)
            && self.cx.typed.coerced.get(&e.id).is_none_or(|c| c.1)
        {
            let p = p.then(Proj::Deref);
            self.lent.entry(e.id).or_default().push(p);
        }
        match &e.kind {
            ExprKind::Unary(UnOp::RefMut, x) => {
                if let Some(p) = self.place_of(x) {
                    self.lent.entry(e.id).or_default().push(p);
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
                let passed: Vec<Option<Place>> = args.iter().map(|a| self.passed(a)).collect();
                self.borrows(e, &passed);
                self.unstable_in(callee, false);
                for (a, p) in args.iter().zip(&passed) {
                    match (&a.kind, p) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::MethodCall(x, _, _, args) => {
                let receiver = match self.cx.typed.methods.get(&e.id) {
                    Some((_, derefs, by)) => self.receiver(x, *derefs, *by),
                    None => self.passed(x),
                };
                let mut passed = vec![receiver];
                passed.extend(args.iter().map(|a| self.passed(a)));
                self.borrows(e, &passed);
                self.unstable_in(x, true);
                for (a, p) in args.iter().zip(&passed[1..]) {
                    match (&a.kind, p) {
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

    /// The place argument `a` lends `&mut`, if it does, by `&mut` or by
    /// passing on a `&mut` it is.
    fn passed(&self, a: &Expr) -> Option<Place> {
        let ty = self.ty_of(a);
        let shared = self.cx.typed.coerced.get(&a.id).is_some_and(|c| !c.1);
        match (&a.kind, &ty) {
            (ExprKind::Unary(UnOp::RefMut, x), _) => self.place_of(x),
            (_, Type::Ref(_, true, _)) if !shared => self.place_of(a).map(|p| p.then(Proj::Deref)),
            _ => None,
        }
    }

    /// The place receiver `x` lends `&mut`, if it does, got through
    /// `derefs` `*`s and then maybe a `&` or `&mut` (`by`).
    fn receiver(&self, x: &Expr, derefs: usize, by: Option<bool>) -> Option<Place> {
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
        match by {
            Some(true) => place,
            Some(false) => None,
            None => match ty {
                Type::Ref(_, true, _) => place.map(|p| p.then(Proj::Deref)),
                _ => None,
            },
        }
    }

    /// The places call `e` lends `&mut`, as `passed` says: what's known of
    /// them is forgotten when it returns. Whatever it does with them then,
    /// or after, if it keeps a reference, nothing reads them until the
    /// borrow ends, as the borrow checker says, and then they're values
    /// nothing's known of.
    fn borrows(&mut self, e: &Expr, passed: &[Option<Place>]) {
        let lent: Vec<Place> = passed.iter().flatten().cloned().collect();
        if !lent.is_empty() {
            self.lent.entry(e.id).or_default().extend(lent);
        }
    }

    /// Can something be written through a value of type `t`: does it hold
    /// a `&mut`, or might it?
    fn has_mut_ref(&self, t: &Type, seen: &mut HashSet<Type>) -> bool {
        match t {
            Type::Int(_) | Type::Bool | Type::Unit | Type::Never | Type::Fn(_) => false,
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
            Type::Bool => self.leaf(p, &t),
            _ if self.is_scalar(&t) => self.leaf(p, &t),
            Type::Adt(..) | Type::Ref(..) | Type::Box(_) => self.read_obj(p, &t),
            t if has_obj(&t) => self.read_obj(p, &t),
            _ => Val::Other,
        }
    }

    /// The value at place `p`, of type `t`, which has an object: a new one
    /// of a refined variable has its refinement.
    fn read_obj(&mut self, p: &Place, t: &Type) -> Val {
        let (o, new) = self.obj_at(p);
        let v = match t {
            Type::Adt(..) => match self.fields_of(p, t) {
                Some(vals) => Val::Struct(o, vals),
                None => Val::Obj(o),
            },
            Type::Ref(..) | Type::Box(_) => Val::Ref(o, Pointee::Place(self.deref(p.clone()))),
            // Its length is its type's, whatever's been done to it.
            Type::Array(_, n) => match self.length(n) {
                Val::Int(l) => Val::Array(l),
                _ => Val::Obj(o),
            },
            _ => Val::Obj(o),
        };
        if new && p.path.is_empty() {
            self.assume_invariant(p.root, &v);
        }
        v
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
            let Some(kind) = self.kind(&ft) else {
                vals.push(self.read_place(&p));
                continue;
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

    /// The `i64` or `bool`, of type `t`, at place `p`.
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
    /// What it's in changes with it, up to a `*`: a reference is the same
    /// one whatever's written through it.
    fn kill(&mut self, p: &Place) {
        self.cur.retain(|q, _| !q.within(p));
        let around = |q: &Place| p.within(q) && !p.path[q.path.len()..].contains(&Proj::Deref);
        self.objs.retain(|q, _| !q.within(p) && !around(q));
    }

    /// Place `p` has a new value, `val` if it's known.
    fn write(&mut self, p: &Place, val: Option<Val>, at: Option<Span>) {
        if let Some(span) = at {
            self.invalidate(p, span, "assignment");
        }
        self.kill(p);
        let t = self.place_ty(p);
        self.set(p, &t, val);
    }

    /// What's at `p`, of type `t`, which nothing's known of, is `val`, if
    /// that's known: each `i64` or `bool` in it stands for a new variable of
    /// the logic, equal to its part of `val`, which has its local's
    /// refinement, if it has one.
    fn set(&mut self, p: &Place, t: &Type, val: Option<Val>) {
        let stable = !self.is_unstable(p);
        let kind = match t {
            Type::Adt(..) => {
                if let (Some(Val::Struct(o, vals)), Some(fields)) =
                    (&val, struct_fields(&self.cx.adts, t))
                {
                    if stable {
                        self.objs.insert(p.clone(), *o);
                    }
                    for (j, (v, (_, ft))) in vals.iter().zip(fields).enumerate() {
                        let ft = self.cx.solved(&ft);
                        self.set(&p.then(Proj::Field(j)), &ft, Some(v.clone()));
                    }
                }
                if let (Some(Val::Obj(o)), true) = (&val, stable) {
                    self.objs.insert(p.clone(), *o);
                }
                if let (Some(v), true) = (&val, p.path.is_empty()) {
                    self.assume_invariant(p.root, v);
                }
                return;
            }
            _ if !self.is_scalar(t) && *t != Type::Bool => {
                if let (Some(Val::Ref(o, _) | Val::Obj(o)), true) = (&val, stable) {
                    self.objs.insert(p.clone(), *o);
                }
                if let (Some(v), true) = (&val, p.path.is_empty()) {
                    self.assume_invariant(p.root, v);
                }
                return;
            }
            t => match self.kind(t) {
                Some(kind) => kind,
                None => return,
            },
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
        self.write(&Place::local(id), val, None);
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
        self.predicate(r, val, env).ok()
    }

    fn predicate(&mut self, r: &Refinement, val: &Val, env: Env) -> Result<Formula, Failure> {
        match self.term(&r.pred, val, env)? {
            Val::Bool(f) => Ok(f),
            _ => Err(Failure::Unsupported),
        }
    }

    fn term(&mut self, e: &Expr, binder: &Val, env: Env) -> Result<Val, Failure> {
        let int = |v: Val| match v {
            Val::Int(l) => Ok(l),
            _ => Err(Failure::Unsupported),
        };
        let boolean = |v: Val| match v {
            Val::Bool(f) => Ok(f),
            _ => Err(Failure::Unsupported),
        };
        Ok(match &e.kind {
            ExprKind::Int(n, _) => Val::Int(Lin::constant(i128::from(*n))),
            ExprKind::Char(c) => Val::Int(Lin::constant(i128::from(u32::from(*c)))),
            ExprKind::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            ExprKind::Var(..) => match (
                self.cx.pred_names.get(&e.id).ok_or(Failure::Unsupported)?,
                env,
            ) {
                (PredName::Binder, _) => binder.clone(),
                (PredName::Param(i), Env::Call(args, _)) => {
                    args.get(*i).ok_or(Failure::Unsupported)?.clone()
                }
                (PredName::Param(i), Env::Here) => {
                    let id = *self.params.get(*i).ok_or(Failure::Unsupported)?;
                    self.read_place(&Place::local(id))
                }
                (PredName::Local(id), _) => self.read_place(&Place::local(*id)),
                (PredName::Field(j), Env::Fields(vals, _)) => {
                    vals.get(*j).ok_or(Failure::Unsupported)?.clone()
                }
                (PredName::Param(_), Env::Fields(..)) | (PredName::Field(_), _) => {
                    return Err(Failure::Unsupported);
                }
                (PredName::Const(name), Env::Call(_, args) | Env::Fields(_, args)) => {
                    match args.iter().find(|(p, _)| p == name) {
                        Some((_, t)) => self.length(&t.clone()),
                        None => self.length(&Type::Param(name.clone())),
                    }
                }
                (PredName::Const(name), Env::Here) => self.length(&Type::Param(name.clone())),
            },
            ExprKind::Unary(UnOp::Neg, x) => Val::Int(
                int(self.term(x, binder, env)?)?
                    .scale(-1)
                    .ok_or(Failure::ArithmeticLimit)?,
            ),
            ExprKind::Unary(UnOp::Not, x) => Val::Bool(boolean(self.term(x, binder, env)?)?.not()),
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.term(l, binder, env)?, self.term(r, binder, env)?);
                let reason = match (&a, &b) {
                    (Val::Int(a), Val::Int(b))
                        if *op == BinOp::Mul
                            && a.as_constant().is_none()
                            && b.as_constant().is_none() =>
                    {
                        Failure::Unsupported
                    }
                    (Val::Int(_), Val::Int(_)) => Failure::ArithmeticLimit,
                    _ => Failure::Unsupported,
                };
                binary(*op, a, b).ok_or(reason)?
            }
            // A measure of what's named, through the `*`s the checker
            // found: the `&self` it's given is the last reference.
            ExprKind::MethodCall(x, ..) => {
                let (symbol, derefs) = self
                    .cx
                    .pred_measures
                    .get(&e.id)
                    .ok_or(Failure::Unsupported)?
                    .clone();
                let v = self.term(x, binder, env)?;
                let recv = self.autoref(v, derefs, Some(false));
                self.measure(&symbol, recv)
            }
            _ => return Err(Failure::Unsupported),
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
        let goal = self.predicate(r, val, env);
        let outcome = if !self.reachable {
            Ok(())
        } else {
            match &goal {
                Ok(g) => self.proves(Some(g)),
                Err(reason) => Err(*reason),
            }
        };
        let Err(reason) = outcome else {
            return Ok(());
        };
        let failure = self.proof_failure(reason, goal.as_ref().ok(), Some(val), &r.text());
        Err(failure.attach(Error::ty(
            span,
            format!("can't prove `{}` for {what}", r.text()),
        )))
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
            let args = self.adt_args(t);
            self.need(r, &vals[j], Env::Fields(vals, &args), span, what)?;
        }
        Ok(())
    }

    /// `v`, the function's result, at `span`, must have its refinement.
    fn result(&mut self, v: Val, span: Span) -> Result<(), Error> {
        if self.closures > 0 {
            return Ok(());
        }
        if let Some((r, name)) = self.ret.clone() {
            let entry = self.entry.clone();
            let what = format!("the result of `{name}`");
            self.need(&r, &v, Env::Call(&entry, &[]), span, what)?;
        }
        Ok(())
    }

    /// `v`, with what each reference in it points to read now, `depth`
    /// references deep, rather than when it's needed.
    fn settled(&mut self, v: Val, depth: usize) -> Val {
        match v {
            Val::Ref(o, Pointee::Place(q)) if depth > 0 => {
                let to = self.read_place(&q);
                let to = self.settled(to, depth - 1);
                Val::Ref(o, Pointee::Val(Box::new(to)))
            }
            Val::Struct(o, vals) if depth > 0 => {
                let vals = vals
                    .into_iter()
                    .map(|v| self.settled(v, depth - 1))
                    .collect();
                Val::Struct(o, vals)
            }
            v => v,
        }
    }

    /// The state to run each branch from.
    fn snapshot(&self) -> Snap {
        Snap {
            facts: self.facts.len(),
            cur: self.cur.clone(),
            objs: self.objs.clone(),
            reachable: self.reachable,
            declared: self.declared.len(),
            invalidated: self.invalidated.clone(),
        }
    }

    /// Goes back to `start`.
    fn restore(&mut self, start: &Snap) {
        self.facts.truncate(start.facts);
        self.cur = start.cur.clone();
        self.objs = start.objs.clone();
        self.reachable = start.reachable;
        self.declared.truncate(start.declared);
        self.invalidated = start.invalidated.clone();
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
            objs: std::mem::take(&mut self.objs),
            reachable: self.reachable,
            val,
            invalidated: self.invalidated.clone(),
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
            self.objs = o.objs;
            self.objs.retain(|p, _| in_scope.contains(&p.root));
            self.invalidated = o.invalidated;
            return o.val;
        }
        // A place is the same object after, if every branch left it so;
        // else it's one nothing's known of.
        let mut objs = reached[0].objs.clone();
        objs.retain(|p, o| {
            in_scope.contains(&p.root) && reached.iter().all(|r| r.objs.get(p) == Some(o))
        });
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
                    let n = self.fresh(self.kinds[*had as usize].clone());
                    cur.insert(p.clone(), n);
                    new.push((n, vars));
                }
            }
        }
        let result = self.unknown(ty);
        let mut cases = Vec::new();
        for (i, o) in reached.into_iter().enumerate() {
            for event in o.invalidated {
                if !self.invalidated.contains(&event) {
                    self.invalidated.push(event);
                }
            }
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
        self.objs = objs;
        result
    }

    /// `e` has been evaluated: what it lent `&mut` may have changed.
    fn returned(&mut self, e: &Expr) {
        for p in self.lent.get(&e.id).cloned().into_iter().flatten() {
            self.invalidate(&p, e.span, "mutable lending");
            self.kill(&p);
        }
    }

    /// The places assigned anywhere in `e`, or lent `&mut` to a call,
    /// together with the source operations that may change them.
    fn assigned(&self, e: &Expr, out: &mut Vec<Invalidation>) {
        if let ExprKind::Assign(_, place, _) = &e.kind
            && let Some(p) = self.place_of(place)
        {
            out.push(Invalidation::new(p, e.span, "assignment"));
        }
        out.extend(
            self.lent
                .get(&e.id)
                .into_iter()
                .flatten()
                .map(|p| Invalidation::new(p.clone(), e.span, "mutable lending")),
        );
        for c in e.children() {
            self.assigned(c, out);
        }
    }

    /// Each place the loop may change has a value nothing's known of but
    /// its refinements. Keep why its entry facts cannot be assumed.
    fn forget(&mut self, changes: &[Invalidation], loop_span: Span) {
        for change in changes {
            self.invalidate_loop(change, loop_span);
            self.kill(change.place());
        }
    }

    /// `l op r`, an operator at node `site` on integers of type `ty`:
    /// proves it can't panic if it can, and gives its value, which it has
    /// if it didn't panic.
    fn arith(&mut self, site: NodeId, op: BinOp, l: Val, r: Val, ty: &Type) -> Val {
        if op.bitwise() {
            return self.bits(site, op, l, r, ty);
        }
        let (Val::Int(a), Val::Int(b), Some((min, max))) = (l, r, self.range(ty)) else {
            if self.reachable {
                self.operator_proof(
                    site,
                    None,
                    Failure::Unsupported,
                    &format!("`{op}` must be safe for `{ty}`"),
                );
            }
            return self.unknown(ty);
        };
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
            // Not by zero, nor the least value by -1. Whatever else is
            // divided fits: the quotient is no further from 0 than what's
            // divided, but for the least value of a signed type by -1, which
            // in each is one past its greatest. So this suffices of every
            // integer type, and so of a type parameter's, though the logic
            // knows of none of them what its greatest value is, but that
            // it's at least -1 minus its least: it holds in every one.
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
        let missing = if op == BinOp::Mul && a.as_constant().is_none() && b.as_constant().is_none()
        {
            Failure::Unsupported
        } else {
            Failure::ArithmeticLimit
        };
        let condition = if matches!(op, BinOp::Div | BinOp::Rem) {
            format!("the divisor must be nonzero and `{op}` must not overflow `{ty}`")
        } else {
            format!("the result of `{op}` must fit in `{ty}`")
        };
        self.operator_proof(site, goal, missing, &condition);
        match value {
            Some(v) => Val::Int(v),
            None => {
                let result = self.unknown(ty);
                if let (Val::Int(q), Some(k)) = (&result, b.as_constant())
                    && k > 0
                    && let Some(f) = divided(op, &a, k, q)
                {
                    self.fact(f);
                } else if let Val::Int(q) = &result
                    && op == BinOp::Rem
                    && let Some(f) = remainder(&a, &b, q)
                {
                    self.fact(f);
                }
                result
            }
        }
    }

    /// `l op r` for a bitwise operator or a shift at node `site`, on
    /// integers of type `ty`. Only `<<` can panic: by a constant it's
    /// multiplying, so linear, and proved not to as `*` is; by anything
    /// else it isn't proved. The others aren't linear, but for `>>` by a
    /// constant, so their result is a new value, of which the logic knows
    /// what `bitwise` says.
    fn bits(&mut self, site: NodeId, op: BinOp, l: Val, r: Val, ty: &Type) -> Val {
        let (Val::Int(a), Val::Int(b)) = (l, r) else {
            return self.unknown(ty);
        };
        if op == BinOp::Shl {
            let shifted = b
                .as_constant()
                .filter(|k| (0..=MAX_SHIFT).contains(k))
                .and_then(|k| a.scale(1 << k));
            let Some(v) = shifted else {
                if self.reachable {
                    let reason = if b
                        .as_constant()
                        .is_some_and(|k| (0..=MAX_SHIFT).contains(&k))
                    {
                        Failure::ArithmeticLimit
                    } else {
                        Failure::Unsupported
                    };
                    self.operator_proof(
                        site,
                        None,
                        reason,
                        "the shifted value must fit in its integer type",
                    );
                }
                return self.unknown(ty);
            };
            let goal = self.fits(&v, ty);
            self.operator_proof(
                site,
                goal,
                Failure::ArithmeticLimit,
                &format!("the shifted value must fit in `{ty}`"),
            );
            return Val::Int(v);
        }
        let result = self.unknown(ty);
        if let Val::Int(q) = &result
            && let Some(f) = bitwise(op, &a, &b, q)
        {
            self.fact(f);
        }
        result
    }

    /// That `v` is a value of `ty`, if it's known to be, as a goal: for
    /// `-x` and `x as T`, whose values are what's in ℤ.
    fn fits(&mut self, v: &Lin, ty: &Type) -> Option<Formula> {
        let (min, max) = self.range(ty)?;
        Some(Formula::and([Formula::le(&min, v)?, Formula::le(v, &max)?]))
    }

    /// What function `callee` names, if it names one: its symbol.
    /// The type arguments of the function call `e` calls, solved.
    fn call_targs(&self, e: &Expr) -> Vec<Type> {
        let res = match &e.kind {
            ExprKind::MethodCall(..) => self.cx.typed.methods.get(&e.id).map(|m| &m.0),
            ExprKind::Call(callee, _) => self.cx.typed.vars.get(&callee.id),
            _ => self.cx.typed.vars.get(&e.id),
        };
        match res {
            Some(Res::Fn(_, targs)) => targs.iter().map(|t| self.cx.solved(t)).collect(),
            _ => Vec::new(),
        }
    }

    /// The value of length `n`: a `Nat`'s number, or a const parameter's
    /// variable, one for the whole function, as the caller picks it once.
    fn length(&mut self, n: &Type) -> Val {
        let usize = Type::Int(crate::types::IntTy::Usize);
        match n {
            Type::Nat(k) => Val::Int(Lin::constant(i128::from(*k))),
            Type::Param(name) => {
                if let Some(v) = self.consts.get(name) {
                    return v.clone();
                }
                let v = self.unknown(&usize);
                self.consts.insert(name.clone(), v.clone());
                v
            }
            _ => self.unknown(&usize),
        }
    }

    /// What struct type `t`'s type parameters are, by name.
    fn adt_args(&self, t: &Type) -> Vec<(String, Type)> {
        match t {
            Type::Adt(symbol, _, args) => match self.cx.adts.get(symbol) {
                Some(d) => d.params.iter().cloned().zip(args.iter().cloned()).collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

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
        let targs: Vec<(String, Type)> = match &sig {
            Some(sig) => sig
                .type_params
                .iter()
                .cloned()
                .zip(self.call_targs(e))
                .collect(),
            None => Vec::new(),
        };
        if let Some(sig) = &sig {
            for (i, r) in sig.params.iter().enumerate() {
                if let (Some(r), Some(arg)) = (r, args.get(i)) {
                    let what = format!("argument `{}` of `{}`", sig.names[i], sig.name);
                    self.need(r, &vals[i], Env::Call(&vals, &targs), arg.span, what)?;
                }
            }
        }
        if ty == Type::Never {
            self.reachable = false;
        }
        // A measure is what it measures.
        if let Some(s) = symbol
            && self.cx.measures.contains_key(s)
            && let Some(recv) = vals.first()
        {
            return Ok(self.measure(s, recv.clone()));
        }
        let result = self.unknown(&ty);
        // What the two intrinsics that say how big an integer type is give.
        if let (Some(f @ ("core::num::least" | "core::num::greatest")), Val::Int(v)) =
            (symbol, &result)
            && let Some((lo, hi)) = self.range(&ty)
            && let Some(f) = Formula::eq(v, if f.ends_with("least") { &lo } else { &hi })
        {
            self.fact(f);
        }
        if let Some(r) = sig.as_ref().and_then(|s| s.ret.as_ref())
            && let Some(f) = self.pred(r, &result, Env::Call(&vals, &targs))
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
        let v = self.expr_here(e)?;
        self.returned(e);
        Ok(v)
    }

    fn expr_here(&mut self, e: &Expr) -> Result<Val, Error> {
        match &e.kind {
            ExprKind::Int(n, _) => Ok(Val::Int(Lin::constant(i128::from(*n)))),
            ExprKind::Char(c) => Ok(Val::Int(Lin::constant(i128::from(u32::from(*c))))),
            ExprKind::Bool(b) => Ok(Val::Bool(if *b { Formula::True } else { Formula::False })),
            ExprKind::Unit | ExprKind::Str(_) | ExprKind::ByteStr(_) => Ok(Val::Other),
            ExprKind::Var(..) => match self.cx.typed.vars.get(&e.id) {
                Some(Res::Local(id)) => Ok(self.read_place(&Place::local(*id))),
                Some(Res::Fn(symbol, _)) => {
                    self.as_value(symbol, e.span)?;
                    Ok(Val::Other)
                }
                Some(Res::Const(name)) => Ok(self.length(&Type::Param(name.clone()))),
                _ => Ok(Val::Other),
            },
            ExprKind::Array(items) => {
                for item in items {
                    self.expr(item)?;
                }
                Ok(Val::Array(Lin::constant(items.len() as i128)))
            }
            ExprKind::Repeat(v, _) => {
                self.expr(v)?;
                let ty = self.ty_of(e);
                Ok(self.unknown(&ty))
            }
            ExprKind::Unary(UnOp::Neg, x) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let Some(neg) = (match v {
                    Val::Int(a) => a.scale(-1),
                    _ => None,
                }) else {
                    if self.reachable {
                        self.operator_proof(
                            e.id,
                            None,
                            Failure::ArithmeticLimit,
                            &format!("the negated value must fit in `{ty}`"),
                        );
                    }
                    return Ok(self.unknown(&ty));
                };
                let goal = self.fits(&neg, &ty);
                self.operator_proof(
                    e.id,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the negated value must fit in `{ty}`"),
                );
                Ok(Val::Int(neg))
            }
            // The same number, if it fits. A character's is a scalar
            // value, and one fits in a `char` if it's one.
            ExprKind::As(x, _) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let Val::Int(a) = v else {
                    return Ok(self.unknown(&ty));
                };
                if self.ty_of(x) == Type::Char
                    && let Some(law) = Self::scalar(&a)
                {
                    self.laws.push(law);
                }
                let goal = match ty {
                    Type::Char => Self::scalar(&a),
                    _ => self.fits(&a, &ty),
                };
                self.operator_proof(
                    e.id,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the converted value must fit in `{ty}`"),
                );
                Ok(Val::Int(a))
            }
            // Of an integer, the least plus the greatest minus it: each bit
            // flipped, which is linear.
            ExprKind::Unary(UnOp::Not, x) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let flipped = match (v, self.range(&ty)) {
                    (Val::Bool(f), _) => return Ok(Val::Bool(f.not())),
                    (Val::Int(a), Some((min, max))) => min.add(&max).and_then(|m| m.sub(&a)),
                    _ => None,
                };
                Ok(match flipped {
                    Some(v) => Val::Int(v),
                    None => self.unknown(&ty),
                })
            }
            ExprKind::Unary(UnOp::Deref, x) => match self.place_of(e) {
                Some(p) => Ok(self.read_place(&p)),
                None => {
                    let v = self.expr(x)?;
                    Ok(match self.pointee(v) {
                        Val::Other => {
                            let ty = self.ty_of(e);
                            self.unknown(&ty)
                        }
                        v => v,
                    })
                }
            },
            // A reference to a place, or to a value. One to what a
            // reference points to is that reference again, the same
            // object: what a slice's length is, say.
            ExprKind::Unary(op, x) => {
                if *op == UnOp::RefMut {
                    self.no_mut_borrow(x, e.span)?;
                }
                match self.place_of(x) {
                    Some(p) => {
                        if p.path.last() == Some(&Proj::Deref) {
                            let outer = Place {
                                root: p.root,
                                path: p.path[..p.path.len() - 1].to_vec(),
                            };
                            if let Type::Ref(..) = self.place_ty(&outer) {
                                return Ok(self.read_place(&outer));
                            }
                        }
                        Ok(Val::Ref(self.new_obj(), Pointee::Place(p)))
                    }
                    None => {
                        let v = self.expr(x)?;
                        Ok(Val::Ref(self.new_obj(), Pointee::Val(Box::new(v))))
                    }
                }
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
                    // An `Ord` call says nothing the logic knows: only the
                    // machine's comparison of integers does.
                    _ if self.cx.is_ord_call(e.id) => self.unknown(&Type::Bool),
                    (op, a, b) if op.arith() || op.bitwise() => {
                        let ty = self.ty_of(e);
                        self.arith(e.id, *op, a, b, &ty)
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
                self.call(e, symbol.as_deref(), &args, vals)
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
                let recv = match method.cloned() {
                    Some((_, derefs, by)) => {
                        let v = self.expr(x)?;
                        self.autoref(v, derefs, by)
                    }
                    None => self.expr(x)?,
                };
                let mut vals = vec![recv];
                for a in args {
                    vals.push(self.expr(a)?);
                }
                let all: Vec<&Expr> = std::iter::once(&**x).chain(args).collect();
                self.call(e, symbol.as_deref(), &all, vals)
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
            ExprKind::Unsafe(block) => self.expr(block),
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
                let ty = self.ty_of(place);
                let old = |w: &mut Self| match &target {
                    Some(p) => w.read_place(p),
                    None => w.unknown(&ty),
                };
                match (&place.kind, self.cx.typed.vars.get(&place.id)) {
                    (ExprKind::Var(name, _), Some(Res::Local(id))) => {
                        let id = *id;
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = old(self);
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if let Some((r, _)) = self.invariants.get(&id).cloned() {
                            let what = format!("`{name}`");
                            self.need(&r, &new, Env::Here, value.span, what)?;
                        }
                        self.write(&Place::local(id), Some(new), Some(e.span));
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
                                    Val::Struct(_, vals) if derefs == 0 => vals,
                                    _ => match self.unknown(&t) {
                                        Val::Struct(_, vals) => vals,
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
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if i < vals.len() {
                            vals[i] = new.clone();
                            self.need_fields(&t, &vals, Some(i), value.span)?;
                        }
                        if let Some(p) = &target {
                            self.write(p, Some(new), Some(e.span));
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
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if let Some(p) = &target {
                            self.write(p, Some(new), Some(e.span));
                        }
                    }
                }
                Ok(Val::Other)
            }
            ExprKind::While(label, c, body) => {
                let mut changed = Vec::new();
                self.assigned(e, &mut changed);
                self.forget(&changed, e.span);
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
                    self.forget(&changed, e.span);
                } else {
                    self.fact(c.not());
                }
                Ok(Val::Other)
            }
            ExprKind::Loop(label, body) => {
                let mut changed = Vec::new();
                self.assigned(e, &mut changed);
                self.forget(&changed, e.span);
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
                self.forget(&changed, e.span);
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
                            let args = self.adt_args(&ty);
                            self.need(r, &vals[j], Env::Fields(&vals, &args), spans[j], what)?;
                        }
                    }
                }
                Ok(Val::Struct(self.new_obj(), vals))
            }
            ExprKind::Field(x, _) => {
                if let Some(p) = self.place_of(e) {
                    return Ok(self.read_place(&p));
                }
                let mut v = self.expr(x)?;
                let (derefs, i) = self.cx.typed.fields[&e.id];
                for _ in 0..derefs {
                    v = self.pointee(v);
                }
                if let Val::Struct(_, mut vals) = v
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
                    Val::Struct(_, mut vals) if i < vals.len() => Ok(vals.swap_remove(i)),
                    _ => {
                        let ty = self.ty_of(e);
                        Ok(self.unknown(&ty))
                    }
                }
            }
            ExprKind::Move(x) => self.expr(x),
            ExprKind::For(_)
            | ExprKind::IfLet(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Range(..)
            | ExprKind::Try(..)
            | ExprKind::Index(..) => {
                unreachable!("rewritten when paths are resolved")
            }
        }
    }

    /// Binds the variables of `pat`, which matched `s` if it's known, at
    /// `place` if it's one: a variable bound by value to an `i64`, a
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
                    (Some(Val::Struct(_, vals)), _) => vals,
                    (_, Some(p)) => match self.read_place(p) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                    _ => match self.unknown(&t) {
                        Val::Struct(_, vals) => vals,
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
            // What an array's values are isn't followed.
            PatternKind::Variant(_, _, _, pats) | PatternKind::Array(pats) => {
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
            | PatternKind::Int(..)
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

/// `a op b` in the logic, if it's in it.
fn binary(op: BinOp, a: Val, b: Val) -> Option<Val> {
    Some(match (op, a, b) {
        (BinOp::Add, Val::Int(a), Val::Int(b)) => Val::Int(a.add(&b)?),
        (BinOp::Sub, Val::Int(a), Val::Int(b)) => Val::Int(a.sub(&b)?),
        (BinOp::Mul, Val::Int(a), Val::Int(b)) => Val::Int(times(&a, &b)?),
        (op, Val::Int(a), Val::Int(b)) => Val::Bool(compare(op, &a, &b)?),
        (BinOp::Eq, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b)),
        (BinOp::Ne, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b).not()),
        (BinOp::And, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::and([a, b])),
        (BinOp::Or, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::or([a, b])),
        _ => return None,
    })
}

/// Does a value of type `t` have an object (see `Val`): is it anything
/// but an integer, a `bool`, `()` or a function?
fn has_obj(t: &Type) -> bool {
    !matches!(
        t,
        Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never | Type::Fn(_)
    )
}

/// That `a` and `b` are equal, as far as the logic can say.
fn equal(a: &Val, b: &Val) -> Vec<Formula> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Formula::eq(a, b).into_iter().collect(),
        (Val::Bool(a), Val::Bool(b)) => vec![Formula::iff(a.clone(), b.clone())],
        (Val::Struct(_, a), Val::Struct(_, b)) => {
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

/// The greatest constant `k` the logic takes `x << k` and `x >> k` to be
/// linear by, as `x` times or divided by 2^k: the integers of every type
/// are less than 2^64 from 0, so further is as good as infinitely far.
const MAX_SHIFT: i128 = 64;

/// What the logic knows of `q`, the result of `a op b` for a bitwise
/// operator or `>>`. Each part is a lemma of two's complement arithmetic,
/// in which a non-negative number has finitely many 1 bits and a negative
/// one infinitely many, to its left:
///
/// - `a & b` has only bits both have, so it's between 0 and `a` if `a >= 0`,
///   and the same of `b`; and it's negative if both are. If `b` is all
///   ones, `2^m - 1`, a number from 0 to `b` keeps all of its bits.
/// - `a | b` has the bits either has: so it's at least each and at most
///   their sum if both are non-negative (it's `a + b - (a & b)`), and
///   negative if either is.
/// - `a ^ b` is non-negative when they have the same sign, and at most their
///   sum if both are non-negative (it's `a + b - 2·(a & b)`).
/// - `a >> b` is `a` divided by 2^b, rounding down: between 0 and `a`, or
///   `a` and -1. By a constant `k`, `2^k·q <= a <= 2^k·q + 2^k - 1`.
///
/// `None` if a number gets too big.
fn bitwise(op: BinOp, a: &Lin, b: &Lin, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let natural = |x: &Lin| Formula::le(&zero, x);
    let implies = |p: Formula, c: Formula| Formula::or([p.not(), c]);
    let between = |lo: &Lin, x: &Lin, hi: &Lin| {
        Some(Formula::and([Formula::le(lo, x)?, Formula::le(x, hi)?]))
    };
    Some(match op {
        BinOp::BitAnd => {
            let mut parts = vec![
                implies(natural(a)?, between(&zero, q, a)?),
                implies(natural(b)?, between(&zero, q, b)?),
                implies(
                    Formula::and([natural(a)?.not(), natural(b)?.not()]),
                    natural(q)?.not(),
                ),
            ];
            for (x, mask) in [(a, b), (b, a)] {
                if let Some(k) = mask.as_constant()
                    && k >= 0
                    && (k + 1).count_ones() == 1
                {
                    parts.push(implies(between(&zero, x, mask)?, Formula::eq(q, x)?));
                }
            }
            Formula::and(parts)
        }
        BinOp::BitOr => Formula::and([
            implies(
                Formula::and([natural(a)?, natural(b)?]),
                Formula::and([
                    Formula::le(a, q)?,
                    Formula::le(b, q)?,
                    Formula::le(q, &a.add(b)?)?,
                ]),
            ),
            implies(
                Formula::or([natural(a)?.not(), natural(b)?.not()]),
                natural(q)?.not(),
            ),
        ]),
        BinOp::BitXor => Formula::and([
            implies(
                Formula::and([natural(a)?, natural(b)?]),
                Formula::le(q, &a.add(b)?)?,
            ),
            Formula::iff(natural(q)?, Formula::iff(natural(a)?, natural(b)?)),
        ]),
        BinOp::Shr => {
            let minus_one = Lin::constant(-1);
            let mut parts = vec![
                implies(natural(a)?, between(&zero, q, a)?),
                implies(natural(a)?.not(), between(a, q, &minus_one)?),
            ];
            match b.as_constant() {
                Some(k @ 0..MAX_SHIFT) => {
                    let p = 1i128 << k;
                    let pq = q.scale(p)?;
                    parts.push(between(&pq, a, &pq.plus(p - 1)?)?);
                }
                Some(MAX_SHIFT..) => {
                    parts.push(implies(natural(a)?, Formula::eq(q, &zero)?));
                    parts.push(implies(natural(a)?.not(), Formula::eq(q, &minus_one)?));
                }
                _ => {}
            }
            Formula::and(parts)
        }
        _ => return None,
    })
}

/// What `q`, the result of `a / k` or `a % k` (`op`) for a constant `k > 0`,
/// is, which the logic can say, as it's linear: rounding towards zero, the
/// quotient is the `q` with `k·q <= a <= k·q + k - 1` if `a >= 0`, and `k·q
/// - k + 1 <= a <= k·q` if `a <= 0`; the remainder has the sign of `a`, and
/// is less than `k` from 0. `None` if a number gets too big.
fn divided(op: BinOp, a: &Lin, k: i128, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let (pos, neg) = (Formula::le(&zero, a)?, Formula::le(a, &zero)?);
    let (when_pos, when_neg) = match op {
        BinOp::Div => {
            let kq = q.scale(k)?;
            (
                Formula::and([Formula::le(&kq, a)?, Formula::le(a, &kq.plus(k - 1)?)?]),
                Formula::and([Formula::le(&kq.plus(1 - k)?, a)?, Formula::le(a, &kq)?]),
            )
        }
        BinOp::Rem => (
            Formula::and([
                Formula::le(&zero, q)?,
                Formula::le(q, &Lin::constant(k - 1))?,
            ]),
            Formula::and([
                Formula::le(&Lin::constant(1 - k), q)?,
                Formula::le(q, &zero)?,
            ]),
        ),
        _ => return None,
    };
    Some(Formula::and([
        Formula::or([pos.not(), when_pos]),
        Formula::or([neg.not(), when_neg]),
    ]))
}

/// What the logic knows of `q = a % b`, by any `b`, which is linear in
/// all three (lemma L4 of step 28): rounding towards zero, the remainder
/// is less than `b` from 0, no further from 0 than `a`, and 0 or of `a`'s
/// sign. These hold of the integers, and so of every integer type, as `%`
/// gives the integers' remainder whenever it doesn't fail: it's less than
/// `b` from 0, so it fits whatever `b` does, and the one case that fails,
/// the least value by -1, is proved not to happen before this is known
/// (and its remainder, 0, would satisfy it anyway). Nothing for `b = 0`,
/// which fails. `None` if a number gets too big.
fn remainder(a: &Lin, b: &Lin, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let le = Formula::le;
    let (a_pos, a_neg) = (le(&zero, a)?, le(a, &zero)?);
    // Less than `|b|` from 0, at `lo..=hi`, one way or the other of it.
    let cases = [
        (Formula::lt(&zero, b)?, b.scale(-1)?.plus(1)?, b.plus(-1)?),
        (Formula::lt(b, &zero)?, b.plus(1)?, b.scale(-1)?.plus(-1)?),
    ];
    let mut parts = Vec::new();
    for (sign, lo, hi) in cases {
        let when = |p: Formula, c: [Formula; 3]| {
            Formula::or([Formula::and([sign.clone(), p]).not(), Formula::and(c)])
        };
        parts.push(when(a_pos.clone(), [le(&zero, q)?, le(q, &hi)?, le(q, a)?]));
        parts.push(when(a_neg.clone(), [le(&lo, q)?, le(q, &zero)?, le(a, q)?]));
    }
    Some(Formula::and(parts))
}

/// That `pat` matches `s`, if that can be said: `None` for a pattern of a
/// struct or enum, which nothing's known of.
fn pattern_formula(pat: &Pattern, s: &Val) -> Option<Formula> {
    match (&pat.kind, s) {
        (PatternKind::Wild | PatternKind::Bind(..), _) => Some(Formula::True),
        (PatternKind::Int(n, _), Val::Int(l)) => Formula::eq(l, &Lin::constant(*n)),
        (PatternKind::Range(lo, hi, ..), Val::Int(l)) => Some(Formula::and([
            match lo {
                Some(lo) => Formula::le(&Lin::constant(*lo), l)?,
                None => Formula::True,
            },
            match hi {
                Some(hi) => Formula::le(l, &Lin::constant(*hi))?,
                None => Formula::True,
            },
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
