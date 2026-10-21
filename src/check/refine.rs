//! Refinement types: `{ d: int | d != 0 }`, the `int`s that aren't 0 (see
//! docs/08-refinement-types.md).
//!
//! A refinement is the whole type of a parameter, a result or a `let`,
//! never part of another type, and only of an `int` or a `bool`. It isn't
//! in `Type` at all: unification, traits and monomorphisation never see
//! one. What it says is checked by a pass of its own over each body, once
//! its types are known (`Checker::refine`), which keeps track of what's
//! known at each point (*facts*) and proves from them what each
//! refinement needs, with the procedure in `refine/logic.rs`:
//!
//! - an argument has its parameter's refinement, a result its function's,
//!   and a value given to a refined `let`, or assigned to one, its;
//! - an operator can't overflow or divide by zero: one proved not to has
//!   no `panic`, which is how refinements make arithmetic total.
//!
//! Facts come from refined parameters and `let`s, from what calls of
//! refined functions return, from the conditions of `if`, `while`, `&&`,
//! `||` and the patterns of a `match` on an `int`, from what a `let` or an
//! assignment gives a variable, and from operators that returned, as they'd
//! have panicked otherwise. They're about *values*: each time a variable
//! gets a new one, it's a new variable of the logic (*static single
//! assignment*), so a fact, once true, stays true, and only what a program
//! variable stands for changes.
//!
//! Soundness: every fact in the context holds of the values the program
//! has at that point, which is an induction on how the program runs. A fact
//! is added only where it's been proved, where a condition is true, or
//! after an operator returned. A program variable stands for a new
//! variable of the logic whenever it may change: when it's assigned, at the
//! start of each time round a loop that assigns it, and after a branch that
//! assigns it, where it's whichever value the branch taken gave it. A `mut`
//! variable that may change some other way, through a `&mut` to it, or a
//! closure that captures it, gives a new value each time it's read: nothing
//! is known of it but its refinement, which every assignment, inside a
//! closure too, must keep. A refined one can't be borrowed `&mut` at all.
//! A refinement's predicate names only variables that can't change. What a
//! call returns has its function's refinement by induction on how deep the
//! calls go, as for effects: a call that never returns gives nothing to
//! count on. And what's called through a value can't have refined
//! parameters, nor a trait's method, so every call that skips a check is
//! of a function with nothing to check.

mod logic;

use super::Checker;
use super::typed::Res;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::Type;
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

/// What a name in a predicate is.
#[derive(Debug, Clone, Copy)]
pub(super) enum PredName {
    /// The refinement's own, the value refined.
    Binder,
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
                    let kind = super::NoteKind::Var(name.clone());
                    self.note(e.span, kind, Some(t.clone()), Some(def));
                    Ok(t)
                }
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
            proved: HashSet::new(),
            params: f.params.iter().map(|p| p.name.id).collect(),
            ret: None,
            closures: 0,
            loops: Vec::new(),
        };
        w.unstable_in(&f.body);
        let mut ret = None;
        for (i, p) in f.params.iter().enumerate() {
            let ty = w.ty_of_bind(p.name.id);
            if let Some(r) = sig.as_ref().and_then(|s| s.params[i].clone()) {
                w.invariants.insert(p.name.id, (r, p.name.name.clone()));
            }
            w.bind(p.name.id, &ty, None);
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
/// `bool`, as a formula, or anything else, of which nothing is known.
#[derive(Debug, Clone)]
enum Val {
    Int(Lin),
    Bool(Formula),
    Other,
}

/// How a path through a branch ended: what it learned, what each variable
/// stood for at its end, whether it gets there, and its value.
struct Outcome {
    facts: Vec<Formula>,
    cur: HashMap<NodeId, Var>,
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
    /// Which variable of the logic each local `int` or `bool` stands for
    /// here, by its id.
    cur: HashMap<NodeId, Var>,
    /// The refinement of each refined local, with its name.
    invariants: HashMap<NodeId, (Refinement, String)>,
    /// The `mut` locals that may change other than by assignment.
    unstable: HashSet<NodeId>,
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
/// of the function being walked, or a call's arguments.
#[derive(Clone, Copy)]
enum Env<'a> {
    Here,
    Call(&'a [Val]),
}

impl Walk<'_> {
    fn fresh(&mut self, kind: Kind) -> Var {
        self.kinds.push(kind);
        (self.kinds.len() - 1) as Var
    }

    /// A value of type `t` of which nothing is known.
    fn unknown(&mut self, t: &Type) -> Val {
        match t {
            Type::Int => Val::Int(Lin::var(self.fresh(Kind::Int))),
            Type::Bool => Val::Bool(Formula::holds(self.fresh(Kind::Bool))),
            _ => Val::Other,
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

    /// The variables of `e` that are `mut` and may change other than by
    /// assignment: borrowed `&mut`, by `&mut` or a method's `&mut self`, or
    /// captured by a closure.
    fn unstable_in(&mut self, e: &Expr) {
        let local = |w: &Walk, x: &Expr| match (&x.unmoved().kind, w.cx.typed.vars.get(&x.id)) {
            (ExprKind::Var(..), Some(Res::Local(id))) => Some(*id),
            _ => None,
        };
        match &e.kind {
            ExprKind::Unary(UnOp::RefMut, x) => {
                if let Some(id) = local(self, x) {
                    self.unstable.insert(id);
                }
            }
            ExprKind::MethodCall(x, ..) => {
                if let Some((_, 0, Some(true))) = self.cx.typed.methods.get(&e.id)
                    && let Some(id) = local(self, x)
                {
                    self.unstable.insert(id);
                }
            }
            ExprKind::Lambda(l) => {
                for id in self.cx.typed.captures.get(&l.id).into_iter().flatten() {
                    if self.cx.typed.binds.get(id).is_some_and(|b| b.mutable) {
                        self.unstable.insert(*id);
                    }
                }
            }
            _ => {}
        }
        for c in e.children() {
            self.unstable_in(c);
        }
    }

    /// What local `id` is here.
    fn read(&mut self, id: NodeId) -> Val {
        let ty = self.ty_of_bind(id);
        if self.unstable.contains(&id) {
            let v = self.unknown(&ty);
            self.assume_invariant(id, &v);
            return v;
        }
        match self.cur.get(&id) {
            Some(&v) => match self.kinds[v as usize] {
                Kind::Int => Val::Int(Lin::var(v)),
                Kind::Bool => Val::Bool(Formula::holds(v)),
            },
            None => self.unknown(&ty),
        }
    }

    /// Local `id`, of type `ty`, has a new value, `val` if it's known: it
    /// stands for a new variable of the logic, equal to that, which has its
    /// refinement, if it has one.
    fn bind(&mut self, id: NodeId, ty: &Type, val: Option<Val>) {
        let v = match ty {
            Type::Int => self.fresh(Kind::Int),
            Type::Bool => self.fresh(Kind::Bool),
            _ => return,
        };
        self.cur.insert(id, v);
        match val {
            Some(Val::Int(l)) => {
                if let Some(f) = Formula::eq(&Lin::var(v), &l) {
                    self.fact(f);
                }
            }
            Some(Val::Bool(f)) => self.fact(Formula::iff(Formula::holds(v), f)),
            _ => {}
        }
        let now = self.read(id);
        self.assume_invariant(id, &now);
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
                    self.read(id)
                }
                (PredName::Local(id), _) => self.read(*id),
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
    fn snapshot(&self) -> (usize, HashMap<NodeId, Var>, bool) {
        (self.facts.len(), self.cur.clone(), self.reachable)
    }

    /// Runs `go` from `start`, assuming `assume`, and gives how it ended.
    fn branch(
        &mut self,
        start: &(usize, HashMap<NodeId, Var>, bool),
        assume: Option<Formula>,
        go: impl FnOnce(&mut Self) -> Result<Val, Error>,
    ) -> Result<Outcome, Error> {
        self.facts.truncate(start.0);
        self.cur = start.1.clone();
        self.reachable = start.2;
        if let Some(f) = assume {
            self.fact(f);
        }
        let val = go(self)?;
        Ok(Outcome {
            facts: self.facts.split_off(start.0),
            cur: std::mem::take(&mut self.cur),
            reachable: self.reachable,
            val,
        })
    }

    /// Where branches `outcomes`, all from `start`, meet: what's known is
    /// that one of them was taken, and each variable a branch gave a new
    /// value is whichever value that branch gave it. Gives the value, of
    /// type `ty`.
    fn join(
        &mut self,
        start: &(usize, HashMap<NodeId, Var>, bool),
        outcomes: Vec<Outcome>,
        ty: &Type,
    ) -> Val {
        self.facts.truncate(start.0);
        self.cur = start.1.clone();
        let reached: Vec<Outcome> = outcomes.into_iter().filter(|o| o.reachable).collect();
        if reached.is_empty() {
            self.reachable = false;
            return self.unknown(ty);
        }
        self.reachable = start.2;
        if let [_] = &reached[..] {
            let o = reached.into_iter().next().expect("one");
            self.facts.extend(o.facts);
            for (id, v) in o.cur {
                if self.cur.contains_key(&id) {
                    self.cur.insert(id, v);
                }
            }
            return o.val;
        }
        let changed: Vec<NodeId> = start
            .1
            .keys()
            .filter(|id| reached.iter().any(|o| o.cur.get(id) != start.1.get(id)))
            .copied()
            .collect();
        let new: Vec<(NodeId, Var)> = changed
            .iter()
            .map(|id| {
                let kind = self.kinds[start.1[id] as usize];
                (*id, self.fresh(kind))
            })
            .collect();
        let result = self.unknown(ty);
        let mut cases = Vec::new();
        for o in reached {
            let mut parts = o.facts;
            match (&result, &o.val) {
                (Val::Int(r), Val::Int(v)) => parts.extend(Formula::eq(r, v)),
                (Val::Bool(r), Val::Bool(v)) => parts.push(Formula::iff(r.clone(), v.clone())),
                _ => {}
            }
            for &(id, n) in &new {
                let had = o.cur.get(&id).copied().unwrap_or(start.1[&id]);
                parts.extend(Formula::eq(&Lin::var(n), &Lin::var(had)));
            }
            cases.push(Formula::and(parts));
        }
        self.fact(Formula::or(cases));
        for (id, n) in new {
            self.cur.insert(id, n);
        }
        result
    }

    /// The locals assigned anywhere in `e`, by id.
    fn assigned(&self, e: &Expr, out: &mut HashSet<NodeId>) {
        if let ExprKind::Assign(_, place, _) = &e.kind
            && let Some(Res::Local(id)) = self.cx.typed.vars.get(&place.id)
        {
            out.insert(*id);
        }
        for c in e.children() {
            self.assigned(c, out);
        }
    }

    /// Each local in `ids` has a value nothing's known of but its
    /// refinement.
    fn forget(&mut self, ids: &HashSet<NodeId>) {
        let mut ids: Vec<NodeId> = ids
            .iter()
            .copied()
            .filter(|id| self.cur.contains_key(id))
            .collect();
        ids.sort_by_key(|id| id.0);
        for id in ids {
            let ty = self.ty_of_bind(id);
            self.bind(id, &ty, None);
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

    /// Fails if `x`, borrowed `&mut` at `span`, is a refined local.
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
        Ok(())
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
                Some(Res::Local(id)) => Ok(self.read(*id)),
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
            ExprKind::Unary(op, x) => {
                if *op == UnOp::RefMut {
                    self.no_mut_borrow(x, e.span)?;
                }
                self.expr(x)?;
                let ty = self.ty_of(e);
                Ok(self.unknown(&ty))
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
                let mut vals = vec![self.expr(x)?];
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
            ExprKind::Match(scrutinee, arms) => {
                let s = self.expr(scrutinee)?;
                let start = self.snapshot();
                let mut before: Vec<Formula> = Vec::new();
                let mut outcomes = Vec::new();
                for arm in arms {
                    let this = pattern_formula(&arm.pat, &s);
                    let assume =
                        Formula::and(before.iter().cloned().map(Formula::not).chain(this.clone()));
                    let o = self.branch(&start, Some(assume), |w| {
                        w.bind_pattern(&arm.pat, Some(&s));
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
                    let ty = self.ty_of_bind(p.name.id);
                    self.bind(p.name.id, &ty, None);
                }
                let result = self.expr(&l.body);
                self.closures -= 1;
                self.loops = loops;
                self.facts.truncate(start.0);
                self.cur = start.1;
                self.reachable = start.2;
                result?;
                Ok(Val::Other)
            }
            ExprKind::Assign(op, place, value) => {
                let v = self.expr(value)?;
                match (&place.kind, self.cx.typed.vars.get(&place.id)) {
                    (ExprKind::Var(name, _), Some(Res::Local(id))) => {
                        let id = *id;
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = self.read(id);
                                self.arith(e.id, *op, old, v)
                            }
                        };
                        if let Some((r, _)) = self.invariants.get(&id).cloned() {
                            let what = format!("`{name}`");
                            self.need(&r, &new, Env::Here, value.span, what)?;
                        }
                        let ty = self.ty_of_bind(id);
                        self.bind(id, &ty, Some(new));
                    }
                    _ => {
                        self.expr(place)?;
                        if let Some(op) = op {
                            let old = self.unknown(&Type::Int);
                            self.arith(e.id, *op, old, v);
                        }
                    }
                }
                Ok(Val::Other)
            }
            ExprKind::While(label, c, body) => {
                let mut changed = HashSet::new();
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
                self.facts.truncate(start.0);
                self.cur = start.1;
                self.reachable = start.2;
                if broke {
                    self.forget(&changed);
                } else {
                    self.fact(c.not());
                }
                Ok(Val::Other)
            }
            ExprKind::Loop(label, body) => {
                let mut changed = HashSet::new();
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
                self.facts.truncate(start.0);
                self.cur = start.1;
                self.reachable = start.2 && broke;
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
                for i in inits {
                    self.expr(&i.value)?;
                }
                Ok(Val::Other)
            }
            ExprKind::Field(x, _) | ExprKind::Move(x) => {
                self.expr(x)?;
                let ty = self.ty_of(e);
                Ok(self.unknown(&ty))
            }
            ExprKind::For(_) | ExprKind::Range(..) => {
                unreachable!("rewritten when paths are resolved")
            }
        }
    }

    /// Binds the variables of `pat`, which matched `s` if it's known: a
    /// variable that's all of it is equal to it, and others are unknown.
    fn bind_pattern(&mut self, pat: &Pattern, s: Option<&Val>) {
        match &pat.kind {
            PatternKind::Bind(..) => {
                let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                let ty = self.ty_of_bind(id);
                let val = s.filter(|v| !matches!(v, Val::Other)).cloned();
                self.bind(id, &ty, val);
            }
            PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
                pats.iter().for_each(|p| self.bind_pattern(p, None));
            }
            PatternKind::Struct(_, _, fields, _) => {
                fields.iter().for_each(|f| self.bind_pattern(&f.pat, None));
            }
            PatternKind::Move(p) => self.bind_pattern(p, s),
            PatternKind::Wild
            | PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
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
                match init {
                    Some(_) => self.bind_pattern(pat, v.as_ref()),
                    None => self.bind_pattern(pat, None),
                }
                Ok(())
            }
            Stmt::Expr(e) | Stmt::BlockLike(e) => self.expr(e).map(drop),
            Stmt::Fn(_) | Stmt::Type(_) => Ok(()),
        }
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
