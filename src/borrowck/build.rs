//! Lowering a checked function body to MIR (see `mir.rs`), and generating
//! the region constraints its types give as it goes: where a value of one
//! type goes into a place of another, the first must be a subtype of the
//! second (`Regions::subtype`); where something is borrowed, the borrow
//! can't outlive what it borrows; where a function is used, what its
//! signature assumes must hold.
//!
//! Every expression is lowered to *write its value into a place*
//! (`expr_into`), or to *be* a place (`place`), or an operand (`operand`).
//! Each statement's temporaries go out of scope at its end, as in the
//! interpreter: the two have to agree on when memory goes away, or the
//! borrow checker would be checking a different program than the one that
//! runs. That goes for `return`, `break` and `continue` too: each ends the
//! statements and scopes it leaves, innermost first, as the interpreter
//! does as it unwinds to where they go.

use super::mir::*;
use super::regions::{AdtInfo, Regions};
use crate::ast::*;
use crate::check::Checker;
use crate::check::typed::Res;
use crate::error::Span;
use crate::types::{AdtKind, Adts, Region, Subst, Type, struct_fields, variant_fields};
use std::collections::HashMap;

/// A borrow: of `place`, `&mut` or not, for as long as region variable
/// `region` says, made at `point`.
pub(crate) struct Loan {
    pub(crate) place: Place,
    pub(crate) mutable: bool,
    /// Is it a two-phase `&mut`, only reserved until a `StmtKind::Activate`?
    pub(crate) two_phase: bool,
    pub(crate) region: usize,
    pub(crate) point: Point,
    pub(crate) span: Span,
}

/// A closure found in the body: its code, its type there, and where it is.
pub(crate) struct ClosureUse<'a> {
    pub(crate) lambda: &'a Lambda,
    pub(crate) ty: Type,
    pub(crate) span: Span,
}

pub(crate) struct Builder<'a> {
    pub(crate) cx: &'a Checker,
    pub(crate) info: &'a AdtInfo,
    pub(crate) body: Body,
    pub(crate) regions: Regions,
    pub(crate) loans: Vec<Loan>,
    pub(crate) closures: Vec<ClosureUse<'a>>,
    /// Is the body a closure's?
    pub(crate) closure: bool,
    block: Bb,
    /// The locals of each scope being lowered, innermost last.
    scopes: Vec<Scope>,
    /// The temporaries of each statement being lowered, innermost last.
    temps: Vec<Vec<Local>>,
    /// Each variable's local, by its id.
    vars: HashMap<Span, Local>,
    /// The loops being lowered, innermost last.
    loops: Vec<LoopFrame>,
    /// Whether control can get to each block, as far as is known: a block
    /// is lowered after every block that jumps to it but those that jump
    /// back to the start of a loop, which control gets to anyway.
    live: Vec<bool>,
}

/// A scope's locals, and how many statements were being lowered when it
/// started: those it's inside.
struct Scope {
    locals: Vec<Local>,
    temps: usize,
}

/// A loop being lowered: its label, if it has one, where a `continue` and
/// a `break` in it go, where a `break` puts the value it gives a `loop`,
/// and how many scopes and statements it's inside, which those leave the
/// rest of.
struct LoopFrame {
    label: Option<String>,
    head: Bb,
    after: Bb,
    dest: Option<Place>,
    scopes: usize,
    temps: usize,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(cx: &'a Checker, info: &'a AdtInfo, regions: Regions) -> Builder<'a> {
        Builder {
            cx,
            info,
            body: Body {
                locals: Vec::new(),
                blocks: vec![Block::default()],
                params: 0,
            },
            regions,
            loans: Vec::new(),
            closures: Vec::new(),
            closure: false,
            block: Bb(0),
            scopes: vec![Scope {
                locals: Vec::new(),
                temps: 0,
            }],
            temps: Vec::new(),
            vars: HashMap::new(),
            loops: Vec::new(),
            live: vec![true],
        }
    }

    pub(crate) fn adts(&self) -> &'a Adts {
        self.cx.adts()
    }

    /// The function's return place and parameters, of these types, with
    /// regions numbered. Parameters are named by `params`: their names and
    /// ids. Captured variables come after, in the same way.
    pub(crate) fn signature(
        &mut self,
        ret: Type,
        params: &[(String, Span, Type)],
        captured: &[(String, Span, Type)],
    ) {
        self.new_local(ret, LocalKind::Return, None, Span::new(0, 0));
        for (name, id, ty) in params {
            let bind = &self.cx.typed().binds[id];
            let (mutable, mut_span) = (bind.mutable, bind.mut_span);
            let l = self.new_local(ty.clone(), LocalKind::Param, Some(name.clone()), *id);
            self.body.locals[l.0].mutable = mutable;
            self.body.locals[l.0].mut_span = mut_span;
            self.vars.insert(*id, l);
            self.scope().push(l);
        }
        self.body.params = params.len();
        // A closure's copies of what it captured are made for each call,
        // and go away when it returns, as its parameters do.
        for (name, id, ty) in captured {
            let l = self.new_local(ty.clone(), LocalKind::Captured, Some(name.clone()), *id);
            self.vars.insert(*id, l);
            self.scope().push(l);
        }
    }

    /// Lowers `body` as what the function returns, then drops the
    /// parameters, and returns. Code control can't get to, like that after
    /// a `return`, is left out, as it never runs.
    pub(crate) fn lower(&mut self, body: &'a Expr) {
        self.temps.push(Vec::new());
        self.expr_into(body, &Place::local(RETURN));
        self.end_temps(body.span);
        let params = std::mem::take(&mut self.scopes[0].locals);
        let end = Span::new(body.span.end.saturating_sub(1), body.span.end);
        for l in params.into_iter().rev() {
            self.push(StmtKind::Dead(l), end);
        }
        self.terminate(Term::Return);
        self.prune();
    }

    /// Empties the blocks control never gets to, and forgets the loans made
    /// in them: what's left of an expression that a jump in it leaves, like
    /// the `&x` of `f(return 1, &x)`.
    fn prune(&mut self) {
        let mut reached = vec![false; self.body.blocks.len()];
        let mut todo = vec![Bb(0)];
        while let Some(b) = todo.pop() {
            if !std::mem::replace(&mut reached[b.0], true) {
                todo.extend(self.body.successors(b));
            }
        }
        for (block, reached) in self.body.blocks.iter_mut().zip(&reached) {
            if !reached {
                *block = Block::default();
            }
        }
        self.loans.retain(|l| reached[l.point.block.0]);
    }

    /// The innermost scope's locals.
    fn scope(&mut self) -> &mut Vec<Local> {
        &mut self.scopes.last_mut().expect("a scope").locals
    }

    fn push_scope(&mut self) {
        let temps = self.temps.len();
        self.scopes.push(Scope {
            locals: Vec::new(),
            temps,
        });
    }

    fn new_local(&mut self, ty: Type, kind: LocalKind, name: Option<String>, span: Span) -> Local {
        self.body.locals.push(LocalDecl {
            ty,
            kind,
            mutable: kind == LocalKind::Temp || kind == LocalKind::Return,
            name,
            span,
            mut_span: None,
            deferred: false,
        });
        Local(self.body.locals.len() - 1)
    }

    /// A new temporary of type `ty`, for the value of what's at `span`,
    /// which goes out of scope at the end of the statement.
    fn temp(&mut self, ty: Type, span: Span) -> Local {
        let l = self.new_local(ty, LocalKind::Temp, None, span);
        match self.temps.last_mut() {
            Some(temps) => temps.push(l),
            None => self.scope().push(l),
        }
        l
    }

    fn point(&self) -> Point {
        Point {
            block: self.block,
            index: self.body.blocks[self.block.0].stmts.len(),
        }
    }

    fn push(&mut self, kind: StmtKind, span: Span) {
        self.body.blocks[self.block.0]
            .stmts
            .push(Statement { kind, span });
    }

    fn new_block(&mut self) -> Bb {
        self.body.blocks.push(Block::default());
        self.live.push(false);
        Bb(self.body.blocks.len() - 1)
    }

    fn terminate(&mut self, t: Term) {
        let block = &mut self.body.blocks[self.block.0];
        if block.term.is_some() {
            return;
        }
        block.term = Some(t);
        if self.live[self.block.0] {
            for b in self.body.successors(self.block) {
                self.live[b.0] = true;
            }
        }
    }

    /// Can control get to what's being lowered? Not after a jump, until
    /// somewhere another path gets to. Code it can't get to never runs, so
    /// it isn't lowered, or borrow checked.
    fn reachable(&self) -> bool {
        self.live[self.block.0]
    }

    fn goto(&mut self, b: Bb) {
        self.terminate(Term::Goto(b));
    }

    /// Writes `value`, of type `ty`, to `place`, which needs `ty` to be a
    /// subtype of the place's type.
    fn assign(&mut self, place: &Place, value: Rvalue, ty: &Type, span: Span, declare: bool) {
        let want = self.place_ty(place);
        self.regions.subtype(self.info, ty, &want, span);
        self.push(
            StmtKind::Assign {
                place: place.clone(),
                value,
                declare,
            },
            span,
        );
    }

    /// Ends the innermost statement, the one at `span`: its temporaries go
    /// out of scope, the last made first, right after it.
    fn end_temps(&mut self, span: Span) {
        let temps = self.temps.pop().expect("a statement");
        for l in temps.into_iter().rev() {
            self.push(StmtKind::Dead(l), Span::new(span.end, span.end));
        }
    }

    /// Ends the innermost scope, of the block at `span`: its locals go out
    /// of scope, the last declared first, at its `}`.
    fn end_scope(&mut self, span: Span) {
        let locals = self.scopes.pop().expect("a scope").locals;
        let end = Span::new(span.end.saturating_sub(1), span.end);
        for l in locals.into_iter().rev() {
            self.push(StmtKind::Dead(l), end);
        }
    }

    /// The type of expression `e`, as the checker found it, with a new
    /// region variable for each region in it.
    fn ty(&mut self, e: &Expr) -> Type {
        let t = self.cx.typed().exprs.get(&std::ptr::from_ref(e));
        let t = self
            .cx
            .solution(t.expect("the checker saw every expression"));
        self.regions.fresh_regions(&t)
    }

    /// The type of what's at `place`.
    pub(crate) fn place_ty(&self, place: &Place) -> Type {
        let mut t = self.body.locals[place.local.0].ty.clone();
        for p in &place.proj {
            t = project(self.adts(), &t, *p);
        }
        t
    }

    // Expressions.

    /// Lowers `e` to write its value to `dest`.
    fn expr_into(&mut self, e: &'a Expr, dest: &Place) {
        if !self.reachable() {
            return;
        }
        if self.cx.typed().coerced.contains_key(&std::ptr::from_ref(e)) {
            let (o, t) = self.operand(e);
            self.assign(dest, Rvalue::Use(o), &t, e.span, false);
            return;
        }
        self.expr_into_uncoerced(e, dest);
    }

    fn expr_into_uncoerced(&mut self, e: &'a Expr, dest: &Place) {
        if !self.reachable() {
            return;
        }
        let span = e.span;
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => {
                let t = self.ty(e);
                self.assign(dest, Rvalue::Use(Operand::Const), &t, span, false);
            }
            ExprKind::Var(..) | ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _) => {
                let (o, t) = self.operand(e);
                self.assign(dest, Rvalue::Use(o), &t, span, false);
            }
            ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) => {
                let p = self.place(x);
                let (r, t) = self.borrow(&p, *op == UnOp::RefMut, span);
                self.assign(
                    dest,
                    Rvalue::Ref(r, *op == UnOp::RefMut, p),
                    &t,
                    span,
                    false,
                );
            }
            ExprKind::Unary(op, x) => {
                let (o, _) = self.operand(x);
                let t = self.ty(e);
                self.assign(dest, Rvalue::Unary(*op, o), &t, span, false);
            }
            ExprKind::Binary(op @ (BinOp::And | BinOp::Or), l, r) => {
                self.expr_into(l, dest);
                let (rhs, join) = (self.new_block(), self.new_block());
                let test = Operand::Copy(dest.clone());
                match op {
                    BinOp::And => self.terminate(Term::Branch(test, rhs, join)),
                    _ => self.terminate(Term::Branch(test, join, rhs)),
                }
                self.block = rhs;
                self.expr_into(r, dest);
                self.goto(join);
                self.block = join;
            }
            // Looked at where they are, not used up.
            ExprKind::Binary(op @ (BinOp::Eq | BinOp::Ne), l, r) => {
                let l = self.place(l);
                let r = self.place(r);
                let value = Rvalue::Binary(*op, Operand::Copy(l), Operand::Copy(r));
                self.assign(dest, value, &Type::Bool, span, false);
            }
            ExprKind::Binary(op, l, r) => {
                let (l, _) = self.operand(l);
                let (r, _) = self.operand(r);
                let t = self.ty(e);
                self.assign(dest, Rvalue::Binary(*op, l, r), &t, span, false);
            }
            ExprKind::Call(f, args) => {
                let (f, ft) = self.operand(f);
                self.call_into(f, ft, None, args, dest, span);
            }
            // The receiver first: what's before the `.`, through its `*`s,
            // borrowed if the method takes `&self` or `&mut self`. A `&mut`
            // is a two-phase borrow, only reserved until the call, so the
            // arguments can still read what it borrows.
            ExprKind::MethodCall(x, _, _, args) => {
                let (res, derefs, autoref) =
                    self.cx.typed().methods[&std::ptr::from_ref(e)].clone();
                let Res::Fn(symbol, targs) = res else {
                    unreachable!("a method is a function")
                };
                let ft = self.fn_value(&symbol, &targs, span);
                let mut recv = self.place(x);
                for _ in 0..derefs {
                    recv = recv.project(Proj::Deref);
                }
                let receiver = match autoref {
                    Some(m) => {
                        let (r, t) = self.borrow(&recv, m, x.span);
                        let loan = self.loans.len() - 1;
                        self.loans[loan].two_phase = m;
                        let tmp = self.temp(t.clone(), x.span);
                        self.assign(
                            &Place::local(tmp),
                            Rvalue::Ref(r, m, recv),
                            &t,
                            x.span,
                            false,
                        );
                        let o = Operand::Move(Place::local(tmp));
                        (o, t, x.span, m.then_some(loan))
                    }
                    None => {
                        let (o, t) = self.use_place(recv, x.span);
                        (o, t, x.span, None)
                    }
                };
                self.call_into(Operand::Const, ft, Some(receiver), args, dest, span);
            }
            // A path to a method: the method, as a function, maybe called.
            ExprKind::Variant(_, _, _, args, parens)
                if let Some(Res::Fn(symbol, targs)) =
                    self.cx.typed().vars.get(&std::ptr::from_ref(e)) =>
            {
                let ft = self.fn_value(symbol, targs, span);
                match parens {
                    true => self.call_into(Operand::Const, ft, None, args, dest, span),
                    false => self.assign(dest, Rvalue::Use(Operand::Const), &ft, span, false),
                }
            }
            ExprKind::If(c, then, els) => {
                let test = self.condition(c);
                let (yes, no, join) = (self.new_block(), self.new_block(), self.new_block());
                self.terminate(Term::Branch(test, yes, no));
                self.block = yes;
                self.expr_into(then, dest);
                self.goto(join);
                self.block = no;
                match els {
                    Some(els) => self.expr_into(els, dest),
                    None => {
                        let unit = Rvalue::Use(Operand::Const);
                        self.assign(dest, unit, &Type::Unit, span, false);
                    }
                }
                self.goto(join);
                self.block = join;
            }
            ExprKind::While(label, c, body) => {
                self.loop_into(label, Some(c), body, None);
                let unit = Rvalue::Use(Operand::Const);
                self.assign(dest, unit, &Type::Unit, span, false);
            }
            ExprKind::Loop(label, body) => self.loop_into(label, None, body, Some(dest)),
            // The value goes where the function's result does, then
            // everything in scope goes away, the parameters too.
            ExprKind::Return(value) => {
                let ret = Place::local(RETURN);
                match value {
                    Some(v) => self.expr_into(v, &ret),
                    None => {
                        self.assign(&ret, Rvalue::Use(Operand::Const), &Type::Unit, span, false)
                    }
                }
                self.unwind(0, 0, span);
                self.diverge(Term::Return);
            }
            ExprKind::Break(label, value) => {
                let lp = self.target(label);
                let (after, scopes, temps) = (lp.after, lp.scopes, lp.temps);
                if let Some(dest) = lp.dest.clone() {
                    match value {
                        Some(v) => self.expr_into(v, &dest),
                        None => {
                            let unit = Rvalue::Use(Operand::Const);
                            self.assign(&dest, unit, &Type::Unit, span, false);
                        }
                    }
                }
                self.unwind(scopes, temps, span);
                self.diverge(Term::Goto(after));
            }
            ExprKind::Continue(label) => {
                let lp = self.target(label);
                let (head, scopes, temps) = (lp.head, lp.scopes, lp.temps);
                self.unwind(scopes, temps, span);
                self.diverge(Term::Goto(head));
            }
            // The value first, then the place, as in the interpreter.
            ExprKind::Assign(op, place, value) => {
                let (v, vt) = self.operand(value);
                let p = self.place(place);
                match op {
                    Some(op) => {
                        let value = Rvalue::Binary(*op, Operand::Copy(p.clone()), v);
                        self.assign(&p, value, &Type::Int, span, false);
                    }
                    None => self.assign(&p, Rvalue::Use(v), &vt, span, false),
                }
                let unit = Rvalue::Use(Operand::Const);
                self.assign(dest, unit, &Type::Unit, span, false);
            }
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), dest, span),
            ExprKind::StructLit(_, _, inits) => {
                let t = self.ty(e);
                self.regions.well_formed(self.adts(), self.info, &t, span);
                let fields = struct_fields(self.adts(), &t).expect("a struct");
                let mut ops: Vec<Option<Operand>> = vec![None; fields.len()];
                for init in inits {
                    let i = fields.iter().position(|(f, _)| *f == init.name.name);
                    let i = i.expect("the checker found the field");
                    let (o, ot) = self.operand(&init.value);
                    self.regions
                        .subtype(self.info, &ot, &fields[i].1, init.value.span);
                    ops[i] = Some(o);
                }
                let ops = ops.into_iter().map(|o| o.expect("every field")).collect();
                self.assign(dest, Rvalue::Aggregate(ops), &t, span, false);
            }
            ExprKind::Variant(ty, _, _, args, ..) if ty.name == "Box" => {
                let t = self.ty(e);
                let Type::Box(inner) = &t else {
                    unreachable!("`Box::new` makes a box")
                };
                let (o, ot) = self.operand(&args[0]);
                self.regions.subtype(self.info, &ot, inner, args[0].span);
                self.assign(dest, Rvalue::Aggregate(vec![o]), &t, span, false);
            }
            ExprKind::Variant(_, _, variant, args, ..) => {
                let t = self.ty(e);
                self.regions.well_formed(self.adts(), self.info, &t, span);
                let i = variant_index(self.adts(), &t, &variant.name);
                let fields = variant_fields(self.adts(), &t, i);
                let mut ops = Vec::new();
                for (a, f) in args.iter().zip(&fields) {
                    let (o, ot) = self.operand(a);
                    self.regions.subtype(self.info, &ot, f, a.span);
                    ops.push(o);
                }
                self.assign(dest, Rvalue::Aggregate(ops), &t, span, false);
            }
            ExprKind::Match(scrutinee, arms) => self.match_into(scrutinee, arms, dest, span),
            ExprKind::Lambda(l) => {
                let t = self.ty(e);
                let captured = self.cx.typed().captures[&std::ptr::from_ref(&**l)].clone();
                let ops = captured
                    .iter()
                    .map(|id| Operand::Copy(Place::local(self.vars[id])))
                    .collect();
                self.closures.push(ClosureUse {
                    lambda: l,
                    ty: t.clone(),
                    span,
                });
                self.assign(dest, Rvalue::Aggregate(ops), &t, span, false);
            }
        }
    }

    /// A call of `f`, of type `ft`, with arguments `args`, after `first`
    /// if there is one: a method's receiver, as an operand, its type and
    /// where it is, and the two-phase loan it took to activate at the call,
    /// if it did.
    fn call_into(
        &mut self,
        f: Operand,
        ft: Type,
        first: Option<(Operand, Type, Span, Option<usize>)>,
        args: &'a [Expr],
        dest: &Place,
        span: Span,
    ) {
        let Type::Fn(sig) = ft else {
            unreachable!("calling `{ft}`")
        };
        // A higher-ranked function type is called at new lifetimes.
        let vars: Vec<Region> = sig
            .bound
            .iter()
            .map(|_| Region::Var(self.regions.fresh()))
            .collect();
        let (params, ret) = sig.instantiate(&vars);
        let mut params = params.iter();
        let mut ops = Vec::new();
        let mut activate = None;
        if let Some((o, t, at, loan)) = first {
            let p = params.next().expect("a method takes its receiver");
            self.regions.subtype(self.info, &t, p, at);
            ops.push(o);
            activate = loan;
        }
        for (a, p) in args.iter().zip(params) {
            let (o, t) = self.operand(a);
            self.regions.subtype(self.info, &t, p, a.span);
            ops.push(o);
        }
        if let Some(loan) = activate {
            self.push(StmtKind::Activate(loan), span);
        }
        self.assign(dest, Rvalue::Call(f, ops), &ret, span, false);
        // A function that returns a `!` never does.
        if ret == Type::Never {
            self.diverge(Term::Switch(Vec::new()));
        }
    }

    /// A `while c { body }`, or with no `c`, a `loop { body }`, whose
    /// `break`s put the value they give it in `dest`.
    fn loop_into(
        &mut self,
        label: &Option<Label>,
        c: Option<&'a Expr>,
        body: &'a Expr,
        dest: Option<&Place>,
    ) {
        let (head, after) = (self.new_block(), self.new_block());
        self.goto(head);
        self.block = head;
        if let Some(c) = c {
            let test = self.condition(c);
            let inside = self.new_block();
            self.terminate(Term::Branch(test, inside, after));
            self.block = inside;
        }
        self.loops.push(LoopFrame {
            label: label.as_ref().map(|l| l.name.name.clone()),
            head,
            after,
            dest: dest.cloned(),
            scopes: self.scopes.len(),
            temps: self.temps.len(),
        });
        self.temps.push(Vec::new());
        let t = self.ty(body);
        let unit = self.temp(t, body.span);
        self.expr_into(body, &Place::local(unit));
        self.end_temps(body.span);
        self.goto(head);
        self.loops.pop();
        self.block = after;
    }

    /// The loop a `break` or `continue` with `label` goes to: the innermost,
    /// or the one with that label.
    fn target(&self, label: &Option<Ident>) -> &LoopFrame {
        let lp = match label {
            None => self.loops.last(),
            Some(l) => self
                .loops
                .iter()
                .rev()
                .find(|lp| lp.label.as_ref() == Some(&l.name)),
        };
        lp.expect("the checker found the loop")
    }

    /// Ends the statements and scopes inside the first `scopes` scopes and
    /// `temps` statements, innermost first, for a jump at `span` out of
    /// them.
    fn unwind(&mut self, scopes: usize, temps: usize, span: Span) {
        let mut dead = Vec::new();
        let mut t = self.temps.len();
        for scope in self.scopes[scopes..].iter().rev() {
            while t > scope.temps.max(temps) {
                t -= 1;
                dead.extend(self.temps[t].iter().rev());
            }
            dead.extend(scope.locals.iter().rev());
        }
        while t > temps {
            t -= 1;
            dead.extend(self.temps[t].iter().rev());
        }
        for l in dead {
            self.push(StmtKind::Dead(l), span);
        }
    }

    /// Ends the block being lowered with `t`, which never goes on to what
    /// comes next. What does, if anything, is in a new block nothing goes
    /// to, so it isn't lowered.
    fn diverge(&mut self, t: Term) {
        self.terminate(t);
        self.block = self.new_block();
    }

    /// Lowers condition `c` of an `if` or `while`, as a statement of its
    /// own, into a `bool` that lasts as long as the statement around it.
    fn condition(&mut self, c: &'a Expr) -> Operand {
        let test = self.temp(Type::Bool, c.span);
        self.temps.push(Vec::new());
        self.expr_into(c, &Place::local(test));
        self.end_temps(c.span);
        Operand::Copy(Place::local(test))
    }

    /// `e` as an operand, with its type. A place is copied out of, or moved
    /// out of if its type isn't `Copy`. A `&mut` is *reborrowed*: what it
    /// points to is borrowed again, for as long as the new reference is
    /// used, so the old one can be used again after. Anything else is
    /// lowered into a temporary, which is moved out of.
    fn operand(&mut self, e: &'a Expr) -> (Operand, Type) {
        let coerced = self.cx.typed().coerced.get(&std::ptr::from_ref(e)).copied();
        let local_var = |b: &Self| match &e.kind {
            ExprKind::Var(..) => matches!(
                b.cx.typed().vars.get(&std::ptr::from_ref(e)),
                Some(Res::Local(_))
            ),
            _ => true,
        };
        let place = matches!(
            e.kind,
            ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _)
        );
        if place || (e.is_place() && local_var(self)) {
            let p = self.place(e);
            return match coerced {
                Some((derefs, m)) => self.reborrow(&p, derefs, m, e.span),
                None => self.use_place(p, e.span),
            };
        }
        let (o, t) = match &e.kind {
            ExprKind::Var(_, _) => {
                let t = match &self.cx.typed().vars[&std::ptr::from_ref(e)] {
                    Res::Fn(symbol, targs) => self.fn_value(symbol, targs, e.span),
                    Res::Builtin => {
                        let t = self
                            .cx
                            .solution(&self.cx.typed().exprs[&std::ptr::from_ref(e)]);
                        self.regions.fresh_regions(&t)
                    }
                    Res::Local(_) => unreachable!("a place"),
                };
                (Operand::Const, t)
            }
            _ => {
                let t = self.ty(e);
                let tmp = self.temp(t.clone(), e.span);
                self.expr_into_uncoerced(e, &Place::local(tmp));
                (Operand::Move(Place::local(tmp)), t)
            }
        };
        match (coerced, o) {
            (Some((derefs, m)), Operand::Move(p)) => self.reborrow(&p, derefs, m, e.span),
            (_, o) => (o, t),
        }
    }

    /// Place `p`, used as a value at `span`: copied, or a `&mut` reborrowed,
    /// or else moved out of.
    fn use_place(&mut self, p: Place, span: Span) -> (Operand, Type) {
        match self.place_ty(&p) {
            Type::Ref(_, true, _) => self.reborrow(&p, 0, true, span),
            t if self.cx.is_copy(&t) => (Operand::Copy(p), t),
            // Moved into a temporary of its own, so that where it's moved is
            // this expression.
            t => {
                let tmp = self.temp(t.clone(), span);
                let value = Rvalue::Use(Operand::Move(p));
                self.assign(&Place::local(tmp), value, &t, span, false);
                (Operand::Move(Place::local(tmp)), t)
            }
        }
    }

    /// `&*p` or `&mut *p`, for reference `p`, in a new temporary, with
    /// `derefs` more `*`s.
    fn reborrow(&mut self, p: &Place, derefs: usize, mutable: bool, span: Span) -> (Operand, Type) {
        let mut target = p.project(Proj::Deref);
        for _ in 0..derefs {
            target = target.project(Proj::Deref);
        }
        let (r, t) = self.borrow(&target, mutable, span);
        let tmp = self.temp(t.clone(), span);
        let value = Rvalue::Ref(r, mutable, target);
        self.assign(&Place::local(tmp), value, &t, span, false);
        (Operand::Move(Place::local(tmp)), t)
    }

    /// The type of function `symbol` used with type arguments `targs`, with
    /// a new region variable for each of its lifetime parameters, and the
    /// constraints for what its signature assumes: its bounds, and that its
    /// parameter and return types make sense.
    fn fn_value(&mut self, symbol: &str, targs: &[Type], span: Span) -> Type {
        let scheme = self.cx.scheme(symbol);
        let targs: Vec<Type> = targs
            .iter()
            .map(|t| {
                let t = self.cx.solution(t);
                self.regions.fresh_regions(&t)
            })
            .collect();
        let vars: Vec<Region> = scheme
            .lifetimes
            .iter()
            .map(|_| Region::Var(self.regions.fresh()))
            .collect();
        let s = Subst::new(&scheme.params, &targs).with_regions(&scheme.lifetimes, &vars);
        let t = s.apply(&scheme.ty);
        let bounds: Vec<_> = scheme
            .bounds
            .iter()
            .map(|b| match b {
                crate::types::Bound::Region(a, b) => {
                    crate::types::Bound::Region(s.region(a), s.region(b))
                }
                crate::types::Bound::Param(p, r) => {
                    crate::types::Bound::Param(p.clone(), s.region(r))
                }
            })
            .collect();
        for b in bounds {
            match b {
                crate::types::Bound::Region(..) => self.regions.require(&[b], span),
                crate::types::Bound::Param(p, r) => {
                    let arg = s.apply(&Type::Param(p));
                    self.regions.type_outlives(&arg, &r, span);
                }
            }
        }
        if let Type::Fn(sig) = &t {
            for p in sig.params.iter().chain([&*sig.ret]) {
                self.regions.well_formed(self.adts(), self.info, p, span);
            }
        }
        t
    }

    /// Borrows `place`, for a new region: a loan, made here. What's
    /// borrowed must outlive the borrow, and a borrow through a reference
    /// can't outlive that reference's region either.
    fn borrow(&mut self, place: &Place, mutable: bool, span: Span) -> (usize, Type) {
        let r = self.regions.fresh();
        let mut t = self.body.locals[place.local.0].ty.clone();
        for p in &place.proj {
            if let (Proj::Deref, Type::Ref(q, _, _)) = (p, &t) {
                self.regions.outlives(q, &Region::Var(r), span);
            }
            t = project(self.adts(), &t, *p);
        }
        self.regions.type_outlives(&t, &Region::Var(r), span);
        self.loans.push(Loan {
            place: place.clone(),
            mutable,
            two_phase: false,
            region: r,
            point: self.point(),
            span,
        });
        (r, Type::Ref(Region::Var(r), mutable, Box::new(t)))
    }

    /// Where the value of place expression `e` is: a variable, with any
    /// fields and `*`s, and those of any references and boxes a field
    /// looks through. Anything else is lowered into a temporary.
    fn place(&mut self, e: &'a Expr) -> Place {
        let typed = self.cx.typed();
        match &e.kind {
            ExprKind::Var(..) => {
                if let Some(Res::Local(id)) = typed.vars.get(&std::ptr::from_ref(e)) {
                    return Place::local(self.vars[id]);
                }
            }
            ExprKind::Field(x, _) => {
                let (derefs, i) = typed.fields[&std::ptr::from_ref(e)];
                let mut p = self.place(x);
                for _ in 0..derefs {
                    p = p.project(Proj::Deref);
                }
                return p.project(Proj::Field(i));
            }
            ExprKind::Unary(UnOp::Deref, x) => return self.place(x).project(Proj::Deref),
            _ => {}
        }
        let t = self.ty(e);
        let tmp = self.temp(t, e.span);
        self.expr_into(e, &Place::local(tmp));
        Place::local(tmp)
    }

    // Blocks and statements.

    fn block(&mut self, stmts: &'a [Stmt], tail: Option<&'a Expr>, dest: &Place, span: Span) {
        self.push_scope();
        for stmt in stmts {
            if !self.reachable() {
                break;
            }
            match stmt {
                Stmt::Let { pat, ty, init, .. } => self.let_stmt(pat, ty.as_deref(), init.as_ref()),
                Stmt::Expr(e) | Stmt::BlockLike(e) => {
                    self.temps.push(Vec::new());
                    let t = self.ty(e);
                    let value = self.temp(t, e.span);
                    self.expr_into(e, &Place::local(value));
                    self.end_temps(e.span);
                }
                Stmt::Fn(_) | Stmt::Type(_) => {}
            }
        }
        match tail {
            _ if !self.reachable() => {}
            Some(tail) => {
                self.temps.push(Vec::new());
                self.expr_into(tail, dest);
                self.end_temps(tail.span);
            }
            None => {
                let unit = Rvalue::Use(Operand::Const);
                self.assign(dest, unit, &Type::Unit, span, false);
            }
        }
        self.end_scope(span);
    }

    /// `let pat: ann = init;`. A plain variable gets the value; a pattern
    /// takes it apart where it is. If `init` is `&x` of a value that isn't a
    /// place, the temporary holding it lives to the end of the block, as in
    /// the interpreter.
    fn let_stmt(&mut self, pat: &'a Pattern, ann: Option<&TypeExpr>, init: Option<&'a Expr>) {
        let Some(init) = init else {
            let l = self.declare_var(pat, ann);
            let decl = &mut self.body.locals[l.0];
            decl.deferred = true;
            // It has no value yet, as if it had just gone out of scope: not
            // even one from the last time round a loop.
            let span = decl.span;
            self.push(StmtKind::Dead(l), span);
            return;
        };
        if let ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) = &init.kind
            && !x.is_place()
        {
            let t = self.ty(x);
            let kept = self.new_local(t, LocalKind::Temp, None, x.span);
            self.scope().push(kept);
            self.temps.push(Vec::new());
            self.expr_into(x, &Place::local(kept));
            self.end_temps(x.span);
            self.temps.push(Vec::new());
            let mutable = *op == UnOp::RefMut;
            let (r, t) = self.borrow(&Place::local(kept), mutable, init.span);
            let tmp = self.temp(t.clone(), init.span);
            let value = Rvalue::Ref(r, mutable, Place::local(kept));
            self.assign(&Place::local(tmp), value, &t, init.span, false);
            self.declare(pat);
            self.bind(pat, &Place::local(tmp));
            self.end_temps(init.span);
            return;
        }
        self.temps.push(Vec::new());
        match &pat.kind {
            PatternKind::Bind(..) => {
                let l = self.declare_var(pat, ann);
                let t = self.ty(init);
                let tmp = self.temp(t.clone(), init.span);
                self.expr_into(init, &Place::local(tmp));
                let value = match self.cx.is_copy(&t) {
                    true => Operand::Copy(Place::local(tmp)),
                    false => Operand::Move(Place::local(tmp)),
                };
                self.assign(&Place::local(l), Rvalue::Use(value), &t, pat.span, true);
            }
            _ => {
                let p = self.place(init);
                self.inspect(pat, &p);
                self.declare(pat);
                self.bind(pat, &p);
            }
        }
        self.end_temps(init.span);
    }

    /// `match scrutinee { arms }`, into `dest`. The scrutinee is looked at
    /// where it is, then one arm runs, binding its variables from it.
    fn match_into(&mut self, scrutinee: &'a Expr, arms: &'a [Arm], dest: &Place, span: Span) {
        self.temps.push(Vec::new());
        let p = self.place(scrutinee);
        for arm in arms {
            self.inspect(&arm.pat, &p);
        }
        let blocks: Vec<Bb> = arms.iter().map(|_| self.new_block()).collect();
        let join = self.new_block();
        self.terminate(Term::Switch(blocks.clone()));
        for (arm, b) in arms.iter().zip(blocks) {
            self.block = b;
            self.push_scope();
            self.declare(&arm.pat);
            self.bind(&arm.pat, &p);
            self.expr_into(&arm.body, dest);
            self.end_scope(arm.body.span);
            self.goto(join);
        }
        self.block = join;
        self.end_temps(span);
    }

    /// How many references pattern `p` looks through (match ergonomics).
    fn peels(&self, p: &Pattern) -> usize {
        let peels = &self.cx.typed().peels;
        peels.get(&std::ptr::from_ref(p)).copied().unwrap_or(0)
    }

    /// Reads what pattern `p` tests in the value at `place`, so that it
    /// can't be changed while it's being looked at.
    fn inspect(&mut self, p: &Pattern, place: &Place) {
        let mut place = place.clone();
        for _ in 0..self.peels(p) {
            place = place.project(Proj::Deref);
        }
        match &p.kind {
            PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => self.push(StmtKind::Inspect(place), p.span),
            PatternKind::Variant(_, _, v, pats) => {
                self.push(StmtKind::Inspect(place.clone()), p.span);
                let t = self.place_ty(&place);
                let i = variant_index(self.adts(), &t, &v.name);
                for (j, q) in pats.iter().enumerate() {
                    self.inspect(q, &place.project(Proj::Variant(i, j)));
                }
            }
            PatternKind::Struct(_, _, fields, _) => {
                let t = self.place_ty(&place);
                for fp in fields {
                    let j = field_index(self.adts(), &t, &fp.name.name);
                    self.inspect(&fp.pat, &place.project(Proj::Field(j)));
                }
            }
            PatternKind::Or(alts) => alts.iter().for_each(|a| self.inspect(a, &place)),
            PatternKind::Wild | PatternKind::Bind(..) => {}
        }
    }

    /// Declares the variables `p` binds, in the innermost scope.
    fn declare(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Bind(..) => {
                self.declare_var(p, None);
            }
            PatternKind::Variant(_, _, _, pats) => pats.iter().for_each(|q| self.declare(q)),
            PatternKind::Struct(_, _, fields, _) => {
                fields.iter().for_each(|f| self.declare(&f.pat))
            }
            // Each alternative binds the same variables.
            PatternKind::Or(alts) => self.declare(&alts[0]),
            _ => {}
        }
    }

    /// Declares the variable binding pattern `p` binds, if it isn't yet,
    /// with the type annotation `ann` says, lifetimes and all.
    fn declare_var(&mut self, p: &Pattern, ann: Option<&TypeExpr>) -> Local {
        let typed = self.cx.typed();
        let id = typed.sites[&p.name_span()];
        if let Some(l) = self.vars.get(&id) {
            return *l;
        }
        let bind = &typed.binds[&id];
        let t = self.cx.solution(&bind.ty);
        let t = self.regions.fresh_regions(&t);
        let l = self.new_local(t, LocalKind::Var, Some(bind.name.clone()), id);
        self.body.locals[l.0].mutable = bind.mutable;
        self.body.locals[l.0].mut_span = bind.mut_span;
        if let Some(t) = ann.and_then(|t| self.cx.written_type(t)) {
            self.body.locals[l.0].ty = self.regions.renumber(&t);
        }
        self.vars.insert(id, l);
        self.scope().push(l);
        l
    }

    /// Gives the variables `p` binds their values, from the value at
    /// `place`, which `p` matches: moved or copied out of it, or borrowed
    /// from it if they bind by reference.
    fn bind(&mut self, p: &Pattern, place: &Place) {
        let mut place = place.clone();
        for _ in 0..self.peels(p) {
            place = place.project(Proj::Deref);
        }
        match &p.kind {
            PatternKind::Bind(..) => {
                let typed = self.cx.typed();
                let id = typed.sites[&p.name_span()];
                let l = self.vars[&id];
                let dest = Place::local(l);
                match typed.binds[&id].by_ref {
                    None => {
                        let t = self.place_ty(&place);
                        let o = match self.cx.is_copy(&t) {
                            true => Operand::Copy(place),
                            false => Operand::Move(place),
                        };
                        self.assign(&dest, Rvalue::Use(o), &t, p.span, true);
                    }
                    Some(m) => {
                        let (r, t) = self.borrow(&place, m, p.span);
                        self.assign(&dest, Rvalue::Ref(r, m, place), &t, p.span, true);
                    }
                }
            }
            PatternKind::Variant(_, _, v, pats) => {
                let t = self.place_ty(&place);
                let i = variant_index(self.adts(), &t, &v.name);
                for (j, q) in pats.iter().enumerate() {
                    self.bind(q, &place.project(Proj::Variant(i, j)));
                }
            }
            PatternKind::Struct(_, _, fields, _) => {
                let t = self.place_ty(&place);
                for fp in fields {
                    let j = field_index(self.adts(), &t, &fp.name.name);
                    self.bind(&fp.pat, &place.project(Proj::Field(j)));
                }
            }
            // Whichever alternative matched binds the variables.
            PatternKind::Or(alts) if !alts.is_empty() && has_binds(p) => {
                let blocks: Vec<Bb> = alts.iter().map(|_| self.new_block()).collect();
                let join = self.new_block();
                self.terminate(Term::Switch(blocks.clone()));
                for (alt, b) in alts.iter().zip(blocks) {
                    self.block = b;
                    self.bind(alt, &place);
                    self.goto(join);
                }
                self.block = join;
            }
            _ => {}
        }
    }
}

/// Does `p` bind any variables?
fn has_binds(p: &Pattern) -> bool {
    !p.binds().is_empty()
}

/// The type of what's at projection `p` of a value of type `t`.
pub(crate) fn project(adts: &Adts, t: &Type, p: Proj) -> Type {
    match (p, t) {
        (Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => (**inner).clone(),
        (Proj::Field(i), t) => struct_fields(adts, t).expect("a struct")[i].1.clone(),
        (Proj::Variant(v, i), t) => variant_fields(adts, t, v)[i].clone(),
        (p, t) => unreachable!("{p:?} of `{t}`"),
    }
}

/// The index of variant `name` of enum type `t`.
fn variant_index(adts: &Adts, t: &Type, name: &str) -> usize {
    let Type::Adt(symbol, ..) = t else {
        unreachable!("variant of `{t}`")
    };
    match &adts[symbol].kind {
        AdtKind::Enum(variants) => variants.iter().position(|(v, _)| v == name),
        AdtKind::Struct(_) => None,
    }
    .expect("the checker found the variant")
}

/// The index of field `name` of struct type `t`.
fn field_index(adts: &Adts, t: &Type, name: &str) -> usize {
    let fields = struct_fields(adts, t).expect("a struct");
    let i = fields.iter().position(|(f, _)| f == name);
    i.expect("the checker found the field")
}
