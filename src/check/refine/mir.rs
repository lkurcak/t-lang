//! The refinement checker on MIR (docs/65-refinements-on-mir.md): the
//! walk of `Walk`, over each body's MIR, as the borrow checker built it,
//! rather than over its AST. What it knows, proves and assumes is what the
//! walk over the AST does, by the same functions; only how it gets around
//! the body differs, and how it finds places:
//!
//! - A variable or parameter is the place of its binding, as on the AST,
//!   so predicates name it the same way; a temporary, or the result, is no
//!   place but a value (`Slot`), as an expression's is on the AST, whatever
//!   borrows or projections of it MIR takes (`Walk::locate`).
//! - The blocks are walked in reverse postorder, the first of two branches
//!   first. Where blocks meet, what's known is what the AST's `join` says
//!   of branches: what all of them knew (the facts they share, from where
//!   they parted), and that one of them was taken. A branch's condition,
//!   or the pattern of a `match`'s arm, is what's known along its edge.
//! - At a loop's head (`Body::heads`), from the edges into it from
//!   outside: what it changes is forgotten, but for its frame, and its
//!   invariant is proved, then assumed; along each edge back to it, its
//!   invariant is proved (lemma I1 of docs/54-loop-invariants.md).
//! - A `&mut` given straight to a call (an operand of it) is lent to the
//!   call: what's known of the place is forgotten once it returns, when
//!   its postconditions are learned. Any other `&mut` forgets it where
//!   it's made.
//! - A closure is walked where it's made, from what's known there, as on
//!   the AST, or, if it's given straight to a function, at the call, after
//!   the other arguments, counting on what they say of its parameters.

use super::*;
use crate::mir::{
    Agg, Bb, Body, Const, Key, Local, LocalKind, LoopNote, Notes, Operand, Pat, Point, Rvalue,
    Shape, Statement, StmtKind, StmtNote, Term,
};

/// What a local that's no variable holds, on MIR: a temporary's value,
/// as an expression's is on the AST, and, if it's a borrow, `&mut` or not,
/// of a place, which place.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Slot {
    val: Val,
    borrows: Option<(bool, Place)>,
}

/// The bodies of the function being walked and of its closures.
type Bodies = [(Key, Body, Notes)];

/// A body being walked.
struct Frame<'b> {
    body: &'b Body,
    notes: &'b Notes,
    /// Its closure, its first parameter, if it's a closure's body.
    env: Option<Local>,
    /// Each local that's an operand of a call: where the call is, and
    /// which operand it is.
    operand_of: HashMap<Local, (Point, usize)>,
    /// Each local that a borrow is made into: `&mut` or not, of where.
    borrow_of: HashMap<Local, (bool, crate::mir::Place)>,
    /// The blocks control gets to, in reverse postorder, the first of two
    /// successors first.
    rpo: Vec<Bb>,
    /// Each block's place in `rpo`.
    order: Vec<usize>,
}

impl<'b> Frame<'b> {
    fn new(body: &'b Body, notes: &'b Notes) -> Frame<'b> {
        let env = (body.locals.get(1)).and_then(|l| (l.kind == LocalKind::Env).then_some(Local(1)));
        let mut operand_of = HashMap::new();
        let mut borrow_of = HashMap::new();
        for (b, block) in body.blocks.iter().enumerate() {
            for (index, s) in block.stmts.iter().enumerate() {
                let StmtKind::Assign { place, value, .. } = &s.kind else {
                    continue;
                };
                match value {
                    Rvalue::Call(_, ops) => {
                        let at = Point {
                            block: Bb(b),
                            index,
                        };
                        for (i, o) in ops.iter().enumerate() {
                            if let Some(p) = o.place()
                                && p.proj.is_empty()
                            {
                                operand_of.insert(p.local, (at, i));
                            }
                        }
                    }
                    Rvalue::Ref(_, m, q) if place.proj.is_empty() => {
                        borrow_of.insert(place.local, (*m, q.clone()));
                    }
                    _ => {}
                }
            }
        }
        let rpo = reverse_postorder(body);
        let mut order = vec![usize::MAX; body.blocks.len()];
        for (i, b) in rpo.iter().enumerate() {
            order[b.0] = i;
        }
        Frame {
            body,
            notes,
            env,
            operand_of,
            borrow_of,
            rpo,
            order,
        }
    }

    /// The binding local `l` is, if it's a variable or a parameter.
    fn id(&self, l: Local) -> Option<NodeId> {
        self.notes.ids.get(l.0).copied().flatten()
    }

    /// The loop whose head is `b`, if it's one's.
    fn loop_at(&self, b: Bb) -> Option<&'b LoopNote> {
        self.notes.loops.iter().find(|lp| lp.head == b)
    }

    /// The invariant of the loop whose head is `b`, if it has one.
    fn invariant(&self, b: Bb) -> Option<&'b Expr> {
        self.body.blocks[b.0]
            .stmts
            .iter()
            .find_map(|s| match s.kind {
                StmtKind::Holds(r) => self.body.refinements[r].parts.first().map(|(_, e)| e),
                _ => None,
            })
    }

    /// The call at `at`: what's called and its operands.
    fn call_at(&self, at: Point) -> Option<(&'b Operand, &'b [Operand])> {
        let s = self.body.blocks[at.block.0].stmts.get(at.index)?;
        match &s.kind {
            StmtKind::Assign {
                value: Rvalue::Call(f, ops),
                ..
            } => Some((f, ops)),
            _ => None,
        }
    }
}

/// The text of what `text`, a place's, is `n` fields in: `s.a` of `s.a.b`
/// and 1.
fn outer_text(text: &str, n: usize) -> &str {
    let mut t = text;
    for _ in 0..n {
        match t.rfind('.') {
            Some(i) => t = &t[..i],
            None => break,
        }
    }
    t
}

/// The blocks of `body` control gets to, in reverse postorder: each
/// before those it goes on to, but along an edge back to a loop's head;
/// of a block's successors, the first first (an `if`'s `then` before its
/// `else`, a `match`'s arms in order), as the AST's walk takes them.
fn reverse_postorder(body: &Body) -> Vec<Bb> {
    let n = body.blocks.len();
    let mut seen = vec![false; n];
    let mut post = Vec::new();
    let succ = |b: Bb| {
        let mut s = body.successors(b);
        s.reverse();
        s
    };
    let mut stack = vec![(Bb(0), succ(Bb(0)), 0)];
    seen[0] = true;
    while let Some((b, next, i)) = stack.last_mut() {
        if let Some(&s) = next.get(*i) {
            *i += 1;
            if !seen[s.0] {
                seen[s.0] = true;
                stack.push((s, succ(s), 0));
            }
        } else {
            post.push(*b);
            stack.pop();
        }
    }
    post.reverse();
    post
}

/// What's known on one path to a point: what the walk's state is, there.
#[derive(Clone)]
struct State {
    facts: Vec<Formula>,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    invalidated: Vec<Invalidation>,
    slots: HashMap<Local, Slot>,
    pinned: Vec<(crate::mir::Place, Val)>,
    dead: HashSet<NodeId>,
}

/// Where a MIR place's value is, to the walk: at a place, or a value at
/// none, of its type.
enum Loc {
    Place(Place),
    Val(Val, Type),
}

impl Checker {
    /// The walk over the MIR of the body of `f`, and its closures'.
    pub(super) fn walk_mir(&self, f: &FnDecl) -> Walked {
        let before = work();
        let mut failed = Vec::new();
        let mut known = None;
        let (result, (proved, failures)) = match self.mir_walk(f, &mut failed, &mut known) {
            Ok(found) => (Ok(()), found),
            Err(e) => (Err(e), Default::default()),
        };
        Walked {
            failed,
            result,
            known,
            proved,
            failures,
            work: since(before),
        }
    }

    /// `refine_walk`, over MIR.
    fn mir_walk(
        &self,
        f: &FnDecl,
        failed: &mut Vec<Error>,
        known_value: &mut Option<Known>,
    ) -> Result<Found, Error> {
        let id = self.current.expect("in a function");
        let symbol = self.fns[id.0].symbol.clone();
        let sig = self.refined.get(&symbol).cloned();
        let bodies = self.refine_mir.borrow();
        let key = Key::Fn(symbol.clone());
        let Some((_, body, notes)) = bodies.iter().find(|(k, ..)| *k == key) else {
            return self.refine_walk(f, failed, known_value);
        };
        let fr = Frame::new(body, notes);
        let mut w = self.new_walk(f);
        w.unstable_mir(&fr, &bodies);
        for (i, p) in f.params.iter().enumerate() {
            if let Some(r) = sig.as_ref().and_then(|s| s.params[i].clone()) {
                w.declare(p.name.id, &r, &p.name.name);
            }
            w.bind(p.name.id, None);
        }
        for p in &f.params {
            let v = w.read_place(&Place::local(p.name.id));
            let v = w.settled(v, ENTRY_DEPTH);
            w.entry.push(v);
        }
        if let Some(sig) = &sig
            && let Some(r) = &sig.ret
        {
            w.ret = Some((r.clone(), sig.name.clone()));
        }
        if let Some(sig) = &sig
            && !sig.ensures.is_empty()
        {
            w.ensures = Some((sig.ensures.clone(), sig.name.clone()));
        }
        let done = w.walk_cfg(&fr, &bodies, f.is_const());
        failed.append(&mut w.errors);
        done?;
        *known_value = w.known.take();
        let proved = std::mem::take(&mut w.proved);
        let failures = std::mem::take(&mut w.failures);
        Ok((proved, failures))
    }
}

impl Walk<'_> {
    /// Finds the places that may change other than by assignment, as
    /// `unstable_in` does on the AST: those patterns bind by `&mut` (each
    /// `match`'s scrutinee, or `let`'s value, that's a place), and each
    /// variable a closure captures that's `mut` or holds a `&mut`, in this
    /// body or its closures'.
    fn unstable_mir(&mut self, fr: &Frame, bodies: &Bodies) {
        for mp in &fr.notes.mut_bound {
            if let Some(p) = self.static_place(fr, mp) {
                self.unstable.insert(p);
            }
        }
        for block in &fr.body.blocks {
            for s in &block.stmts {
                let StmtKind::Assign {
                    value: Rvalue::Aggregate(Agg::Closure(l), ..),
                    ..
                } = &s.kind
                else {
                    continue;
                };
                for id in self.cx.typed.captures.get(l).into_iter().flatten() {
                    let Some(b) = self.cx.typed.binds.get(id) else {
                        continue;
                    };
                    let ty = self.cx.solved(&b.ty);
                    if b.mutable || self.has_mut_ref(&ty, &mut HashSet::new()) {
                        self.unstable.insert(Place::local(*id));
                    }
                }
                let key = Key::Closure(*l);
                if let Some((_, body, notes)) = bodies.iter().find(|(k, ..)| *k == key) {
                    let inner = Frame::new(body, notes);
                    self.unstable_mir(&inner, bodies);
                }
            }
        }
    }

    /// What's known here, taken out of the walk.
    fn take_state(&mut self) -> State {
        State {
            facts: std::mem::take(&mut self.facts),
            cur: std::mem::take(&mut self.cur),
            objs: std::mem::take(&mut self.objs),
            reachable: self.reachable,
            invalidated: std::mem::take(&mut self.invalidated),
            slots: std::mem::take(&mut self.slots),
            pinned: std::mem::take(&mut self.pinned),
            dead: std::mem::take(&mut self.dead),
        }
    }

    /// What's known here: `s`.
    fn put_state(&mut self, s: State) {
        self.facts = s.facts;
        self.cur = s.cur;
        self.objs = s.objs;
        self.reachable = s.reachable;
        self.invalidated = s.invalidated;
        self.slots = s.slots;
        self.pinned = s.pinned;
        self.dead = s.dead;
    }

    /// Walks the blocks of body `fr`, from what's known now. Its result
    /// is checked where it returns, but a closure's (`Walk::closures`).
    fn walk_cfg(&mut self, fr: &Frame, bodies: &Bodies, konst: bool) -> Result<(), Error> {
        let mut pending: HashMap<Bb, Vec<State>> = HashMap::new();
        pending.insert(Bb(0), vec![self.take_state()]);
        for &b in &fr.rpo {
            let Some(states) = pending.remove(&b) else {
                continue;
            };
            let state = self.join_states(fr, states);
            self.put_state(state);
            if let Some(lp) = fr.loop_at(b) {
                self.mir_loop_start(fr, lp)?;
            }
            let block = &fr.body.blocks[b.0];
            for (index, s) in block.stmts.iter().enumerate() {
                let at = Point { block: b, index };
                self.mir_stmt(fr, bodies, at, s)?;
            }
            let at = Point {
                block: b,
                index: block.stmts.len(),
            };
            let edges = self.mir_term(fr, at, block.term.as_ref(), konst)?;
            for (to, state) in edges {
                if fr.order[to.0] <= fr.order[b.0] {
                    self.put_state(state);
                    self.back_edge(fr, b, to)?;
                } else {
                    pending.entry(to).or_default().push(state);
                }
            }
        }
        Ok(())
    }

    /// Where paths `states` meet: as the AST's `join` says of branches,
    /// from the facts they all share, where they parted.
    fn join_states(&mut self, fr: &Frame, mut states: Vec<State>) -> State {
        // What's out of scope on any path is out of scope here, as it is
        // where the AST's walk joins branches, one or more.
        let dead: HashSet<NodeId> = states.iter().flat_map(|s| s.dead.iter().copied()).collect();
        for s in &mut states {
            s.cur.retain(|p, _| !dead.contains(&p.root));
            s.objs.retain(|p, _| !dead.contains(&p.root));
            s.dead.clear();
        }
        if states.len() == 1 {
            return states.pop().expect("one");
        }
        let shared = (0..)
            .take_while(|&i| {
                let first = states[0].facts.get(i);
                first.is_some() && states.iter().all(|s| s.facts.get(i) == first)
            })
            .count();
        let reached: Vec<&State> = states.iter().filter(|s| s.reachable).collect();
        // The values temporaries have in every path taken: the same, or
        // to be joined, as a branch's value is.
        let mut slots: HashMap<Local, Slot> = HashMap::new();
        let mut differ: Vec<Local> = Vec::new();
        if let Some(first) = reached.first() {
            let mut locals: Vec<&Local> = first.slots.keys().collect();
            locals.sort();
            for l in locals {
                let each: Option<Vec<&Slot>> = reached.iter().map(|s| s.slots.get(l)).collect();
                let Some(each) = each else {
                    continue;
                };
                if each.iter().all(|s| *s == each[0]) {
                    slots.insert(*l, each[0].clone());
                } else {
                    let borrows = match each.iter().all(|s| s.borrows == each[0].borrows) {
                        true => each[0].borrows.clone(),
                        false => None,
                    };
                    slots.insert(
                        *l,
                        Slot {
                            val: Val::Other,
                            borrows,
                        },
                    );
                    differ.push(*l);
                }
            }
        }
        let pinned: Vec<(crate::mir::Place, Val)> = match reached.first() {
            Some(first) => (first.pinned.iter())
                .filter(|p| reached.iter().all(|s| s.pinned.contains(p)))
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        let values: Vec<(Type, Vec<Val>)> = (differ.iter())
            .map(|l| {
                let t = self.cx.solved(&fr.body.locals[l.0].ty);
                let vals = (states.iter())
                    .map(|s| s.slots.get(l).map_or(Val::Other, |s| s.val.clone()))
                    .collect();
                (t, vals)
            })
            .collect();
        self.facts = states[0].facts[..shared].to_vec();
        self.cur = HashMap::new();
        self.objs = HashMap::new();
        self.reachable = true;
        self.invalidated = Vec::new();
        let start = self.snapshot();
        let outcomes: Vec<Outcome> = (states.into_iter())
            .map(|s| Outcome {
                facts: s.facts[shared..].to_vec(),
                cur: s.cur,
                objs: s.objs,
                reachable: s.reachable,
                val: Val::Other,
                invalidated: s.invalidated,
            })
            .collect();
        let joined = self.join_with(&start, outcomes, values);
        for (l, v) in differ.into_iter().zip(joined) {
            if let Some(s) = slots.get_mut(&l) {
                s.val = v;
            }
        }
        let mut state = self.take_state();
        state.slots = slots;
        state.pinned = pinned;
        state
    }

    /// The head of loop `lp`, from outside it: as `loop_head` says.
    fn mir_loop_start(&mut self, fr: &Frame, lp: &LoopNote) -> Result<(), Error> {
        let mut changed = Vec::new();
        let mut changes = Vec::new();
        for &b in &lp.blocks {
            if fr.order[b.0] == usize::MAX {
                continue;
            }
            for s in &fr.body.blocks[b.0].stmts {
                let StmtKind::Assign {
                    place,
                    value,
                    declare,
                } = &s.kind
                else {
                    continue;
                };
                if !*declare
                    && fr.id(place.local).is_some() | (Some(place.local) == fr.env)
                    && let Some(p) = self.static_place(fr, place)
                {
                    changed.push(Invalidation::new(p.clone(), s.span, "assignment"));
                    changes.push((p, None));
                }
                if let Rvalue::Ref(_, true, q) = value
                    && let Some(p) = self.static_place(fr, q)
                {
                    // Lent to a call, by the call.
                    let call = (place.proj.is_empty())
                        .then(|| fr.operand_of.get(&place.local))
                        .flatten();
                    let span = match call {
                        Some((at, _)) => fr.body.blocks[at.block.0].stmts[at.index].span,
                        None => s.span,
                    };
                    changed.push(Invalidation::new(p.clone(), span, "mutable lending"));
                    let keeps = match place.proj.is_empty() {
                        true => self.lent_keeps(fr, place.local, &p),
                        false => None,
                    };
                    changes.push((p, keeps));
                }
            }
        }
        let frames = self.frames_of(&changes, &changed);
        self.loop_start(&changed, &frames, fr.invariant(lp.head), lp.span)
    }

    /// The measures a call keeps of `p`, if `local`, a `&mut` of it, is
    /// given straight to it, and nothing else it's given lends `p`
    /// (`Walk::keeps`).
    fn lent_keeps(&mut self, fr: &Frame, local: Local, p: &Place) -> Option<Vec<String>> {
        let &(at, i) = fr.operand_of.get(&local)?;
        let (f, ops) = fr.call_at(at)?;
        let Operand::Const(Const::Fn(symbol, targs)) = f else {
            return None;
        };
        let lends = |w: &Self, o: &Operand| {
            let q = o.place().filter(|q| q.proj.is_empty())?;
            match fr.borrow_of.get(&q.local) {
                Some((true, q)) => w.static_place(fr, q),
                _ => None,
            }
        };
        let same = ops
            .iter()
            .filter(|o| lends(self, o).as_ref() == Some(p))
            .count();
        if same != 1 {
            return None;
        }
        let targs = targs.iter().map(|t| self.cx.solved(t)).collect();
        let (symbol, targs) = self.callee_at(symbol, targs);
        let tys: Vec<Option<Type>> = ops.iter().map(|o| Some(self.operand_ty(fr, o))).collect();
        self.keeps_at(&symbol, targs, &tys, i)
    }

    /// Along an edge from `from` back to loop head `head`: its invariant
    /// must hold, for the next time round.
    fn back_edge(&mut self, fr: &Frame, from: Bb, head: Bb) -> Result<(), Error> {
        let (Some(lp), Some(inv)) = (fr.loop_at(head), fr.invariant(head)) else {
            return Ok(());
        };
        let r = Refinement::whole(inv.clone());
        let (span, what) = match lp.continues.get(&from) {
            Some(span) => (*span, "the loop's invariant, at this `continue`"),
            None => (lp.end, "the loop's invariant, at the end of its body"),
        };
        self.need(&r, &Val::Other, Env::Here, span, what.to_string())
    }

    fn mir_stmt(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        at: Point,
        s: &Statement,
    ) -> Result<(), Error> {
        match &s.kind {
            StmtKind::Assign {
                place,
                value,
                declare,
            } => self.mir_assign(fr, bodies, at, s.span, place, value, *declare),
            StmtKind::Dead(l) => {
                self.pinned.clear();
                self.mir_dead(fr, *l, s.span)
            }
            StmtKind::Inspect(_)
            | StmtKind::Consume(..)
            | StmtKind::Refine(..)
            | StmtKind::Holds(_)
            | StmtKind::Kept(_)
            | StmtKind::Drop(..)
            | StmtKind::Free(_) => Ok(()),
        }
    }

    /// Local `l` goes out of scope, or, if it's declared with no value, is
    /// declared (`let x;`), where the AST's walk binds it to none.
    fn mir_dead(&mut self, fr: &Frame, l: Local, span: Span) -> Result<(), Error> {
        let decl = &fr.body.locals[l.0];
        let Some(id) = fr.id(l) else {
            self.slots.remove(&l);
            return Ok(());
        };
        // The parameters go out of scope as the body returns, but the
        // result's refinement and what it ensures are about them then.
        if decl.kind == LocalKind::Param {
            return Ok(());
        }
        if decl.deferred && span == decl.span {
            if let Some((r, name)) = self.refined_let(id) {
                self.need(&r, &Val::Other, Env::Here, span, format!("`{name}`"))?;
                self.declare(id, &r, &name);
            }
            self.bind(id, None);
            return Ok(());
        }
        self.dead.insert(id);
        Ok(())
    }

    /// The refinement `let` declares variable `id` with, if it does, with
    /// its name.
    fn refined_let(&self, id: NodeId) -> Option<(Refinement, String)> {
        let typed = &self.cx.typed;
        let (_, r) = (self.cx.refined_lets.iter())
            .find(|(pat, _)| typed.sites.get(pat).copied().unwrap_or(**pat) == id)?;
        let name = typed
            .binds
            .get(&id)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        Some((r.clone(), name))
    }

    #[allow(clippy::too_many_arguments)]
    fn mir_assign(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        at: Point,
        span: Span,
        place: &crate::mir::Place,
        value: &Rvalue,
        declare: bool,
    ) -> Result<(), Error> {
        let slot =
            place.proj.is_empty() && fr.id(place.local).is_none() && fr.env != Some(place.local);
        if !declare && !slot {
            self.pinned.clear();
        }
        // A variable's first value, by a `let` or a pattern.
        if declare
            && place.proj.is_empty()
            && let Some(id) = fr.id(place.local)
        {
            return self.mir_bind(fr, at, span, place.local, id, value);
        }
        let v = self.rvalue(fr, bodies, at, span, place, value)?;
        if slot {
            let borrows = match value {
                Rvalue::Ref(_, m, q) => self.static_place(fr, q).map(|p| (*m, p)),
                Rvalue::Use(Operand::Copy(q) | Operand::Move(q)) if q.proj.is_empty() => {
                    self.slots.get(&q.local).and_then(|s| s.borrows.clone())
                }
                _ => None,
            };
            self.slots.insert(place.local, Slot { val: v, borrows });
            return Ok(());
        }
        self.mir_write(fr, at, span, place, v)
    }

    /// A variable's first value, `value`: as `bind_pattern` binds one.
    fn mir_bind(
        &mut self,
        fr: &Frame,
        at: Point,
        span: Span,
        l: Local,
        id: NodeId,
        value: &Rvalue,
    ) -> Result<(), Error> {
        if let Rvalue::Ref(_, m, q) = value {
            return self.bind_ref(fr, at, span, id, *m, q);
        }
        let v = match value {
            Rvalue::Use(o) => self.operand(fr, o, span)?,
            _ => Val::Other,
        };
        let val = (!matches!(v, Val::Other)).then_some(v);
        if let Some((r, name)) = self.refined_let(id) {
            let from = match value {
                Rvalue::Use(Operand::Copy(q) | Operand::Move(q)) if q.proj.is_empty() => {
                    fr.body.locals[q.local.0].span
                }
                _ => span,
            };
            let v = val.clone().unwrap_or(Val::Other);
            self.need(&r, &v, Env::Here, from, format!("`{name}`"))?;
            self.declare(id, &r, &name);
        }
        // Without a refinement, it declares what its value's type has
        // (`targs::widen`).
        if let Some(v) = &val
            && !self.decl.contains_key(&id)
        {
            let t = self.ty_of_bind(id);
            let known = self.knowledge(v, &t);
            let tree = targs::widen(&known);
            if !tree.is_empty() {
                self.decl.insert(id, tree);
            }
        }
        self.bind(id, val);
        // `let r = &p;`: `*r` is `p`, while `r` can be used.
        if !fr.body.locals[l.0].mutable
            && let Rvalue::Use(Operand::Copy(q) | Operand::Move(q)) = value
            && q.proj.is_empty()
            && let Some(Slot {
                borrows: Some((false, p)),
                ..
            }) = self.slots.get(&q.local)
        {
            let p = p.clone();
            self.alias.insert(id, p);
        }
        Ok(())
    }

    /// Variable `id` bound by reference, `&mut` if `m`, to place `q`, by a
    /// pattern (match ergonomics): as `bind_pattern` does.
    fn bind_ref(
        &mut self,
        fr: &Frame,
        at: Point,
        span: Span,
        id: NodeId,
        m: bool,
        q: &crate::mir::Place,
    ) -> Result<(), Error> {
        let name = (self.cx.typed.binds.get(&id)).map_or(String::new(), |b| b.name.clone());
        if let Some(StmtNote::BindMut { outer, of_struct }) = fr.notes.stmts.get(&at) {
            // What it's in has no place of its own here, so what's written
            // through it would go unchecked: not where anything is declared
            // or carried of the value (lemma P2 of
            // docs/62-t-chess-findings.md).
            for (pspan, unpeeled, peels, shape) in outer {
                let mut after = unpeeled.clone();
                for _ in 0..*peels {
                    after = after.project(crate::mir::Proj::Deref);
                }
                let place = match self.locate(fr, &after) {
                    Loc::Place(p) => Some(p),
                    Loc::Val(..) => None,
                };
                let placeless = match shape {
                    Shape::Variant | Shape::Array | Shape::Or => true,
                    Shape::Fields => place.is_none(),
                };
                if !placeless {
                    continue;
                }
                let v = match self.locate(fr, unpeeled) {
                    Loc::Place(p) => Some(self.read_place(&p)),
                    Loc::Val(Val::Other, _) => None,
                    Loc::Val(v, _) => Some(v),
                };
                if self.said_of(place.as_ref(), v.as_ref()) {
                    return Err(Error::ty(
                        *pspan,
                        format!(
                            "can't bind `{name}` by `&mut`: what it's in has a refinement, \
                             which a write through the reference could break"
                        ),
                    ));
                }
            }
            if *of_struct && let Some((crate::mir::Proj::Field(i), prefix)) = q.proj.split_last() {
                let outer = crate::mir::Place {
                    local: q.local,
                    proj: prefix.to_vec(),
                };
                let t = self.mir_ty(fr, &outer);
                if let Some(why) = self.refined_fields(&t).and_then(|f| f.constrained(*i)) {
                    return Err(Error::ty(
                        span,
                        format!(
                            "can't bind `{name}` by `&mut`: {why}, which a write through the \
                             reference could break"
                        ),
                    ));
                }
            }
        }
        let widen = |w: &mut Self, r: &Val| {
            if !w.decl.contains_key(&id) {
                let t = w.ty_of_bind(id);
                let known = w.knowledge(r, &t);
                let tree = targs::widen(&known);
                if !tree.is_empty() {
                    w.decl.insert(id, tree);
                }
            }
        };
        match (m, self.locate(fr, q)) {
            (false, Loc::Place(p)) => {
                self.bind(id, None);
                self.alias.insert(id, p);
            }
            // As `let r = &mut p;`: what's written through it must be
            // what's declared of `p` (lemma P2 of
            // docs/62-t-chess-findings.md).
            (true, Loc::Place(p)) => {
                let o = self.new_obj();
                let tree = targs::under(&self.place_decl(&p), 0);
                if !tree.is_empty() {
                    self.trees.insert(o, tree);
                }
                let r = Val::Ref(o, Pointee::Place(p));
                widen(self, &r);
                self.bind(id, Some(r));
            }
            // A `&` to a value known, a variant's field.
            (false, Loc::Val(v, _)) if !matches!(v, Val::Other) => {
                let o = self.new_obj();
                let r = Val::Ref(o, Pointee::Val(Box::new(v)));
                widen(self, &r);
                self.bind(id, Some(r));
            }
            _ => self.bind(id, None),
        }
        Ok(())
    }

    /// An assignment of `new` to `place`, which isn't a temporary's: as the
    /// AST's walk does an `Assign`.
    fn mir_write(
        &mut self,
        fr: &Frame,
        at: Point,
        span: Span,
        place: &crate::mir::Place,
        new: Val,
    ) -> Result<(), Error> {
        let (vspan, text) = match fr.notes.stmts.get(&at) {
            Some(StmtNote::Assign { value, place }) => (*value, place.clone()),
            _ => (span, String::new()),
        };
        let ty = self.mir_ty(fr, place);
        let target = self.static_place(fr, place);
        if let Some(p) = &target
            && p.path.is_empty()
        {
            let id = p.root;
            let name = (self.cx.typed.binds.get(&id)).map_or(String::new(), |b| b.name.clone());
            if let Some((r, _)) = self.invariants.get(&id).cloned() {
                let what = format!("`{name}`");
                self.need(&r, &new, Env::Here, vspan, what)?;
            }
            let d = self.decl.get(&id).cloned().unwrap_or_default();
            let what = format!("`{name}`");
            let written = self.written_tree(&new, &ty);
            self.need_tree(&new, &ty, &d, Some(&written), vspan, &what)?;
            self.write(p, Some(new), Some(span));
            return Ok(());
        }
        let last = place.proj.last().copied();
        let prefix = crate::mir::Place {
            local: place.local,
            proj: place.proj[..place.proj.len().saturating_sub(1)].to_vec(),
        };
        if let Some(crate::mir::Proj::Field(i)) = last {
            let (t, mut vals, declared) = match &target {
                Some(p) => {
                    let (q, _) = p.field_of().expect("a field");
                    let t = self.place_ty(&q);
                    let vals = self.fields_of(&q, &t).unwrap_or_default();
                    (t, vals, None)
                }
                None => {
                    let (v, decl, t) = self.lvalue_mir(fr, &prefix);
                    let vals = match v {
                        Val::Struct(_, vals) => vals,
                        _ => match self.unknown(&t) {
                            Val::Struct(_, vals) => vals,
                            _ => Vec::new(),
                        },
                    };
                    let declared = self.field_decl(&decl, &t, i, None);
                    (t, vals, Some(declared))
                }
            };
            if let Some(d) = &declared {
                let what = format!("what's written to `{text}`");
                let written = self.written_tree(&new, &ty);
                self.need_tree(&new, &ty, d, Some(&written), vspan, &what)?;
            }
            if let Some(p) = &target {
                self.keep_declared(p, &text, &new, &ty, vspan)?;
            }
            if i < vals.len() {
                vals[i] = new.clone();
                self.need_fields(&t, &vals, Some(i), vspan)?;
            }
            if let Some(p) = &target {
                self.around_before(p);
                self.write(p, Some(new), Some(span));
                self.around_after(p, vspan)?;
                self.keep_invariant(p, vspan)?;
            }
            return Ok(());
        }
        // Through a reference that isn't a place's: what it says of what it
        // points to, what's written must have.
        if target.is_none()
            && last == Some(crate::mir::Proj::Deref)
            && let Val::Ref(o, _) = self.read_mir(fr, &prefix)
            && let Some(tree) = self.trees.get(&o).cloned()
        {
            let d = targs::below(&tree, 0);
            let what = format!("what's written through `{text}`");
            let written = self.written_tree(&new, &ty);
            self.need_tree(&new, &ty, &d, Some(&written), vspan, &what)?;
        }
        if let Some(p) = &target {
            self.keep_declared(p, &text, &new, &ty, vspan)?;
            self.around_before(p);
            self.write(p, Some(new), Some(span));
            self.around_after(p, vspan)?;
            self.keep_invariant(p, vspan)?;
        }
        Ok(())
    }

    /// The value of rvalue `rv`, written to `dest`.
    #[allow(clippy::too_many_arguments)]
    fn rvalue(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        at: Point,
        span: Span,
        dest: &crate::mir::Place,
        rv: &Rvalue,
    ) -> Result<Val, Error> {
        match rv {
            // `-n`: the same number, which must fit (as the AST's walk does
            // `-` of a literal).
            Rvalue::Use(o) if let Some(StmtNote::Negated(site)) = fr.notes.stmts.get(&at) => {
                let v = self.operand(fr, o, span)?;
                let ty = self.mir_ty(fr, dest);
                let goal = match &v {
                    Val::Int(a) => self.fits(a, &ty),
                    _ => None,
                };
                self.operator_proof(
                    *site,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the negated value must fit in `{ty}`"),
                );
                Ok(v)
            }
            Rvalue::Use(o) => self.operand(fr, o, span),
            Rvalue::Ref(_, m, q) => self.mir_borrow(fr, at, span, dest, *m, q),
            Rvalue::Unary(UnOp::Neg, o, site) => {
                let v = self.operand(fr, o, span)?;
                let ty = self.mir_ty(fr, dest);
                let Some(neg) = (match v {
                    Val::Int(a) => a.scale(-1),
                    _ => None,
                }) else {
                    if self.reachable {
                        self.operator_proof(
                            *site,
                            None,
                            Failure::ArithmeticLimit,
                            &format!("the negated value must fit in `{ty}`"),
                        );
                    }
                    return Ok(self.unknown(&ty));
                };
                let goal = self.fits(&neg, &ty);
                self.operator_proof(
                    *site,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the negated value must fit in `{ty}`"),
                );
                Ok(Val::Int(neg))
            }
            Rvalue::Unary(_, o, _) => {
                let v = self.operand(fr, o, span)?;
                let ty = self.mir_ty(fr, dest);
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
            Rvalue::Binary(op, l, r, site) => {
                let a = self.operand(fr, l, span)?;
                let b = self.operand(fr, r, span)?;
                Ok(match (op, a, b) {
                    _ if self.cx.is_ord_call(*site) => self.unknown(&Type::Bool),
                    (op, a, b) if op.arith() || op.bitwise() => {
                        let ty = self.mir_ty(fr, dest);
                        self.arith(*site, *op, a, b, &ty)
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
            // A closure used as a function.
            Rvalue::Cast(o, Type::Fn(_), _) => {
                self.operand(fr, o, span)?;
                Ok(Val::Other)
            }
            // The same number, if it fits. A character's is a scalar
            // value, and one fits in a `char` if it's one.
            Rvalue::Cast(o, ty, site) => {
                let v = self.operand(fr, o, span)?;
                let ty = self.cx.solved(ty);
                let Val::Int(a) = v else {
                    return Ok(self.unknown(&ty));
                };
                if self.operand_ty(fr, o) == Type::Char
                    && let Some(law) = Self::scalar(&a)
                {
                    self.laws.push(law);
                }
                let goal = match ty {
                    Type::Char => Self::scalar(&a),
                    _ => self.fits(&a, &ty),
                };
                self.operator_proof(
                    *site,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the converted value must fit in `{ty}`"),
                );
                Ok(Val::Int(a))
            }
            Rvalue::Aggregate(agg, ty, ops) => {
                let ty = self.cx.solved(ty);
                self.aggregate(fr, bodies, at, span, dest, *agg, &ty, ops)
            }
            Rvalue::Call(f, ops) => self.mir_call(fr, bodies, at, span, dest, f, ops),
        }
    }

    /// A struct, variant, box, array or closure, of type `ty`, from `ops`.
    #[allow(clippy::too_many_arguments)]
    fn aggregate(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        at: Point,
        span: Span,
        dest: &crate::mir::Place,
        agg: Agg,
        ty: &Type,
        ops: &[Operand],
    ) -> Result<Val, Error> {
        let spans: Vec<Span> = match fr.notes.stmts.get(&at) {
            Some(StmtNote::Operands(spans, _)) => spans.clone(),
            _ => Vec::new(),
        };
        let span_of = |i: usize| spans.get(i).copied().unwrap_or(span);
        if let Agg::Closure(l) = agg {
            // Given to a function: walked at the call.
            if dest.proj.is_empty()
                && let Some((call, _)) = fr.operand_of.get(&dest.local)
                && let Some((Operand::Const(Const::Fn(..)), _)) = fr.call_at(*call)
            {
                self.deferred.insert(dest.local, (l, ty.clone()));
                return Ok(Val::Other);
            }
            for (i, o) in ops.iter().enumerate() {
                self.operand(fr, o, span_of(i))?;
            }
            self.walk_closure(bodies, l, ty)?;
            return Ok(Val::Other);
        }
        let mut vals = Vec::new();
        for (i, o) in ops.iter().enumerate() {
            vals.push(self.operand(fr, o, span_of(i))?);
        }
        match agg {
            Agg::Struct if matches!(ty, Type::Tuple(_)) => Ok(Val::Struct(self.new_obj(), vals)),
            Agg::Struct => {
                let names = struct_fields(&self.cx.adts, ty).unwrap_or_default();
                let spans: Vec<Span> = (0..names.len()).map(span_of).collect();
                if let Some(fields) = self.refined_fields(ty).cloned() {
                    for (j, r) in fields.refs.iter().enumerate() {
                        if let Some(r) = r {
                            let what = format!("field `{}` of `{}`", fields.names[j], fields.name);
                            let args = self.adt_args(ty);
                            self.need(r, &vals[j], Env::Fields(&vals, &args), spans[j], what)?;
                        }
                    }
                }
                let field_names: Vec<String> = names.iter().map(|(n, _)| n.clone()).collect();
                self.keep_fields_inv(ty, None, &vals, &field_names, &spans)?;
                Ok(Val::Struct(self.new_obj(), vals))
            }
            Agg::Variant(v) => {
                let spans: Vec<Span> = match vals.is_empty() {
                    true => vec![span],
                    false => (0..vals.len()).map(span_of).collect(),
                };
                self.keep_fields_inv(ty, Some(v), &vals, &[], &spans)?;
                let o = self.new_obj();
                self.enum_parts(o, ty, Some((v, vals)));
                Ok(Val::Obj(o))
            }
            Agg::Box => {
                let v = vals.into_iter().next().unwrap_or(Val::Other);
                Ok(Val::Ref(self.new_obj(), Pointee::Val(Box::new(v))))
            }
            Agg::Array => {
                let n = i128::try_from(vals.len()).expect("a length");
                let v = Val::Array(self.new_obj(), Lin::constant(n));
                self.elements(&v, ty, &vals);
                Ok(v)
            }
            Agg::Repeat => {
                let v = self.unknown(ty);
                self.elements(&v, ty, &vals);
                Ok(v)
            }
            Agg::Closure(_) => unreachable!("walked above"),
        }
    }

    /// Walks closure `l`, of type `ty`, from what's known here, as the
    /// AST's walk does a closure literal: then goes back to what's known
    /// here, as making one runs nothing.
    fn walk_closure(&mut self, bodies: &Bodies, l: NodeId, ty: &Type) -> Result<(), Error> {
        let key = Key::Closure(l);
        let Some((_, body, notes)) = bodies.iter().find(|(k, ..)| *k == key) else {
            return Ok(());
        };
        let fr = Frame::new(body, notes);
        let here = self.take_state();
        let declared = self.declared.len();
        let mut start = here.clone();
        start.slots = HashMap::new();
        start.pinned = Vec::new();
        self.put_state(start);
        self.closures += 1;
        let assumed = self.closure_params.remove(&l).unwrap_or_default();
        if let Type::Adt(symbol, ..) = ty {
            self.closure_decls.insert(symbol.clone(), assumed.clone());
        }
        for (i, local) in (2..=body.params).enumerate() {
            let Some(id) = fr.id(Local(local)) else {
                continue;
            };
            if let Some(tree) = assumed.get(i).filter(|t| !t.is_empty()) {
                self.decl.insert(id, tree.clone());
            }
            self.bind(id, None);
        }
        let result = self.walk_cfg(&fr, bodies, false);
        self.closures -= 1;
        self.put_state(here);
        self.declared.truncate(declared);
        result
    }

    /// A `&` or `&mut` (`m`) of `q`, made into `dest`: as the AST's walk
    /// does `&x`, and if it's a `&mut` not given straight to a call, what's
    /// known of the place is forgotten (`Walk::returned`).
    fn mir_borrow(
        &mut self,
        fr: &Frame,
        at: Point,
        span: Span,
        dest: &crate::mir::Place,
        m: bool,
        q: &crate::mir::Place,
    ) -> Result<Val, Error> {
        let lent_to = match dest.proj.is_empty() {
            true => fr.operand_of.get(&dest.local).copied(),
            false => None,
        };
        if m && let Some(StmtNote::Borrow(text)) = fr.notes.stmts.get(&at) {
            let scoped = lent_to.is_some_and(|(call, i)| self.scoped_at(fr, call, i));
            self.no_mut_borrow_mir(fr, q, text, span, scoped)?;
        }
        let v = self.borrow_mir(fr, q);
        if m && lent_to.is_none()
            && let Some(p) = self.static_place(fr, q)
        {
            self.invalidate(&p, span, "mutable lending");
            self.kill(&p);
        }
        Ok(v)
    }

    /// Is operand `i` of the call at `at` for a call-scoped parameter?
    fn scoped_at(&self, fr: &Frame, at: Point, i: usize) -> bool {
        let Some((Operand::Const(Const::Fn(symbol, targs)), _)) = fr.call_at(at) else {
            return false;
        };
        let targs = targs.iter().map(|t| self.cx.solved(t)).collect();
        let (symbol, _) = self.callee_at(symbol, targs);
        self.cx.scoped_params(&symbol).get(i) == Some(&true)
    }

    /// `no_mut_borrow`, of place `q`, written `text`, borrowed `&mut` at
    /// `span`.
    fn no_mut_borrow_mir(
        &mut self,
        fr: &Frame,
        q: &crate::mir::Place,
        text: &str,
        span: Span,
        scoped: bool,
    ) -> Result<(), Error> {
        let place = self.static_place(fr, q);
        if let Some(p) = &place
            && p.path.is_empty()
            && self.invariants.contains_key(&p.root)
            && !scoped
        {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{text}` as `&mut`: its type has a refinement, which a write \
                     through the reference could break"
                ),
            ));
        }
        // Nor a part of one, through fields and boxes: a write to the part
        // is a write to it.
        if let Some(p) = &place
            && !p.path.is_empty()
            && let Some(name) = self.in_refined_place(p)
        {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{text}` as `&mut`: it's in `{name}`, whose type has a \
                     refinement, which a write through the reference could break"
                ),
            ));
        }
        let why = match (scoped, &place) {
            (false, _) => self.constrained_mir(fr, q, text, false),
            (true, None) => self.constrained_mir(fr, q, text, true),
            (true, Some(_)) => None,
        };
        if let Some(why) = why {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{text}` as `&mut`: {why}, which a write through the reference \
                     could break"
                ),
            ));
        }
        Ok(())
    }

    /// `in_refined_local`, of place `p`: the refined local it's in, through
    /// fields and boxes.
    fn in_refined_place(&mut self, p: &Place) -> Option<String> {
        if !self.boxes_only(p) {
            return None;
        }
        self.invariants.get(&p.root).map(|(_, name)| name.clone())
    }

    /// `constrained_field`, of MIR place `q`, written `text`: the field of
    /// a struct a refinement is about that it is, or is in, through fields
    /// and boxes.
    fn constrained_mir(
        &mut self,
        fr: &Frame,
        q: &crate::mir::Place,
        text: &str,
        unplaced: bool,
    ) -> Option<String> {
        let mut k = q.proj.len();
        let mut first = true;
        // How many fields up from `q` this is.
        let mut fields = 0;
        while k > 0 {
            match q.proj[k - 1] {
                crate::mir::Proj::Field(i) => {
                    // Through the `*`s before it, to the struct.
                    let mut s = k - 1;
                    let mut through_ref = false;
                    while s > 0 && q.proj[s - 1] == crate::mir::Proj::Deref {
                        let before = crate::mir::Place {
                            local: q.local,
                            proj: q.proj[..s - 1].to_vec(),
                        };
                        match self.mir_ty(fr, &before) {
                            Type::Box(_) => {}
                            Type::Ref(..) => through_ref = true,
                            _ => return None,
                        }
                        s -= 1;
                    }
                    let st = crate::mir::Place {
                        local: q.local,
                        proj: q.proj[..k - 1].to_vec(),
                    };
                    let t = self.mir_ty(fr, &st);
                    let why = self.refined_fields(&t).and_then(|f| match unplaced {
                        true => f.constrained_unplaced(i),
                        false => f.constrained(i),
                    });
                    if let Some(why) = why {
                        return Some(match first {
                            true => why,
                            false => format!("it's in `{}`: {why}", outer_text(text, fields)),
                        });
                    }
                    fields += 1;
                    if through_ref {
                        return None;
                    }
                    first = false;
                    k = s;
                }
                crate::mir::Proj::Deref => {
                    let before = crate::mir::Place {
                        local: q.local,
                        proj: q.proj[..k - 1].to_vec(),
                    };
                    if !matches!(self.mir_ty(fr, &before), Type::Box(_)) {
                        return None;
                    }
                    first = false;
                    k -= 1;
                }
                crate::mir::Proj::Variant(..) => return None,
            }
        }
        None
    }

    /// The value of a `&` or `&mut` of `q`, as `borrow` says: a reborrow
    /// of what a reference points to is that reference; one of a place
    /// says what's declared there; one of a value at no place, what's
    /// declared of it (`lvalue_mir`).
    fn borrow_mir(&mut self, fr: &Frame, q: &crate::mir::Place) -> Val {
        match self.locate(fr, q) {
            Loc::Place(p) => {
                if p.path.last() == Some(&Proj::Deref) {
                    let outer = Place {
                        root: p.root,
                        path: p.path[..p.path.len() - 1].to_vec(),
                    };
                    if let Type::Ref(..) = self.place_ty(&outer) {
                        return self.read_place(&outer);
                    }
                }
                let o = self.new_obj();
                let tree = targs::under(&self.place_decl(&p), 0);
                if !tree.is_empty() {
                    self.trees.insert(o, tree);
                }
                Val::Ref(o, Pointee::Place(p))
            }
            Loc::Val(..) => {
                if let Some((crate::mir::Proj::Deref, prefix)) = q.proj.split_last() {
                    let outer = crate::mir::Place {
                        local: q.local,
                        proj: prefix.to_vec(),
                    };
                    if let Type::Ref(..) = self.mir_ty(fr, &outer) {
                        return self.read_mir(fr, &outer);
                    }
                }
                let (v, decl, _) = self.lvalue_mir(fr, q);
                let o = self.new_obj();
                let tree = targs::under(&decl, 0);
                if !tree.is_empty() {
                    self.trees.insert(o, tree);
                }
                Val::Ref(o, Pointee::Val(Box::new(v)))
            }
        }
    }

    /// The value at `q`, what's declared of it there, and its type: as
    /// `lvalue` says.
    fn lvalue_mir(&mut self, fr: &Frame, q: &crate::mir::Place) -> (Val, targs::Tree, Type) {
        let t = self.mir_ty(fr, q);
        if let Some(p) = self.static_place(fr, q) {
            let v = self.read_place(&p);
            let decl = self.place_decl(&p);
            return (v, decl, t);
        }
        let Some((last, prefix)) = q.proj.split_last() else {
            let v = self.read_mir(fr, q);
            let known = self.knowledge(&v, &t);
            return (v, targs::widen(&known), t);
        };
        let outer = crate::mir::Place {
            local: q.local,
            proj: prefix.to_vec(),
        };
        match last {
            crate::mir::Proj::Deref => {
                let (r, decl, rt) = self.lvalue_mir(fr, &outer);
                let (v, decl) = self.deref_lvalue(r, &decl, &rt);
                (v, decl, t)
            }
            crate::mir::Proj::Field(i) => {
                let (v, decl, st) = self.lvalue_mir(fr, &outer);
                let vals = match v {
                    Val::Struct(_, vals) => vals,
                    _ => match self.unknown(&st) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                };
                let decl = self.field_decl(&decl, &st, *i, Some(&vals));
                let fv = match vals.get(*i) {
                    Some(v) => v.clone(),
                    None => self.unknown(&t),
                };
                (fv, decl, t)
            }
            crate::mir::Proj::Variant(..) => {
                let v = self.read_mir(fr, q);
                let known = self.knowledge(&v, &t);
                (v, targs::widen(&known), t)
            }
        }
    }

    /// A call of `f` with `ops`, into `dest`: as the AST's walk does one.
    #[allow(clippy::too_many_arguments)]
    fn mir_call(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        at: Point,
        span: Span,
        dest: &crate::mir::Place,
        f: &Operand,
        ops: &[Operand],
    ) -> Result<Val, Error> {
        let ty = self.mir_ty(fr, dest);
        let (symbol, targs, first) = match f {
            Operand::Const(Const::Fn(s, targs)) => {
                let targs: Vec<Type> = targs.iter().map(|t| self.cx.solved(t)).collect();
                (Some(s.clone()), targs, 0)
            }
            Operand::Const(Const::Receiver(_)) => (None, Vec::new(), 1),
            o => {
                self.operand(fr, o, span)?;
                (None, Vec::new(), 0)
            }
        };
        // A const: what's known of its value, or else what its type says,
        // one value for the whole function (`Walk::const_item`).
        if let Some(s) = &symbol
            && ops.is_empty()
            && self.cx.fns.iter().any(|g| g.symbol == *s && g.konst)
        {
            return Ok(self.const_item(s));
        }
        let (spans, texts) = match fr.notes.stmts.get(&at) {
            Some(StmtNote::Operands(spans, texts)) => (spans.clone(), texts.clone()),
            _ => (Vec::new(), Vec::new()),
        };
        let args = &ops[first.min(ops.len())..];
        let spans: Vec<Span> = (first..ops.len())
            .map(|i| spans.get(i).copied().unwrap_or(span))
            .collect();
        let mut vals = Vec::new();
        let mut closures = Vec::new();
        for (i, o) in args.iter().enumerate() {
            let local = o.place().filter(|p| p.proj.is_empty()).map(|p| p.local);
            if let Some((l, cty)) = local.and_then(|l| self.deferred.remove(&l)) {
                vals.push(Val::Other);
                closures.push((i, l, cty));
                continue;
            }
            vals.push(self.operand(fr, o, spans[i])?);
        }
        let places: Vec<Option<Place>> = (args.iter())
            .map(|o| {
                let q = o.place().filter(|q| q.proj.is_empty())?;
                match self.slots.get(&q.local)?.borrows.clone()? {
                    (true, p) => Some(p),
                    (false, _) => None,
                }
            })
            .collect();
        let resolved = symbol.map(|s| self.callee_at(&s, targs));
        if !closures.is_empty() {
            let trees = match &resolved {
                Some((s, t)) => {
                    let idx: Vec<usize> = closures.iter().map(|(i, ..)| *i).collect();
                    self.closure_trees(s, t, &vals, &places, &idx)
                }
                None => HashMap::new(),
            };
            for (i, l, cty) in closures {
                if let Some(t) = trees.get(&i) {
                    self.closure_params.insert(l, t.clone());
                }
                self.walk_closure(bodies, l, &cty)?;
            }
        }
        let site = CallAt {
            ty,
            resolved,
            spans,
            tys: args.iter().map(|o| self.operand_ty(fr, o)).collect(),
            texts,
            coerced: vec![None; args.len()],
            span,
        };
        let (result, post) = self.call_at(&site, vals, places.clone())?;
        // What it lent `&mut` may have changed.
        for p in places.iter().flatten() {
            self.invalidate(p, span, "mutable lending");
            self.kill(p);
        }
        self.post_now(post, span)?;
        Ok(result)
    }

    /// The edges out of the block whose terminator is `term`, at `at`: to
    /// each block, with what's known along the edge.
    fn mir_term(
        &mut self,
        fr: &Frame,
        at: Point,
        term: Option<&Term>,
        konst: bool,
    ) -> Result<Vec<(Bb, State)>, Error> {
        match term {
            None | Some(Term::Unreachable) => Ok(Vec::new()),
            Some(Term::Goto(b)) => Ok(vec![(*b, self.take_state())]),
            Some(Term::Branch(o, yes, no)) => {
                let span = Span::new(0, 0);
                let c = match self.operand(fr, o, span)? {
                    Val::Bool(f) => f,
                    _ => Formula::holds(self.fresh(Kind::Bool)),
                };
                let base = self.take_state();
                let mut out = Vec::new();
                for (to, holds) in [(*yes, true), (*no, false)] {
                    let mut s = base.clone();
                    let f = if holds { c.clone() } else { c.clone().not() };
                    if f != Formula::True {
                        s.facts.push(f);
                    }
                    out.push((to, s));
                }
                Ok(out)
            }
            Some(Term::Match(mp, pats, bbs)) => {
                let s = self.read_mir(fr, mp);
                let mut before: Vec<Formula> = Vec::new();
                let mut assumed = Vec::new();
                for pat in pats {
                    let this = self.pat_formula(pat, &s);
                    // What matching implies but isn't the same as: true of
                    // this arm, so assumed in it, but not false of the arms
                    // after, so not in `before`.
                    let implied = self.pat_implies(pat, &s);
                    let assume = Formula::and(
                        (before.iter().cloned().map(Formula::not))
                            .chain(this.clone())
                            .chain(implied),
                    );
                    assumed.push(assume);
                    before.extend(this);
                }
                let base = self.take_state();
                let mut out = Vec::new();
                for (b, assume) in bbs.iter().zip(assumed) {
                    let mut st = base.clone();
                    if assume != Formula::True {
                        st.facts.push(assume);
                    }
                    st.pinned.push((mp.clone(), s.clone()));
                    out.push((*b, st));
                }
                Ok(out)
            }
            Some(Term::Return) => {
                if self.closures == 0 {
                    let v =
                        (self.slots.get(&crate::mir::RETURN)).map_or(Val::Other, |s| s.val.clone());
                    let (span, tail) = (fr.notes.returns.get(&at.block).copied())
                        .unwrap_or((Span::new(0, 0), true));
                    if tail && konst && self.reachable {
                        self.known = match &v {
                            Val::Int(l) => l.as_constant().map(Known::Int),
                            Val::Bool(Formula::True) => Some(Known::Bool(true)),
                            Val::Bool(Formula::False) => Some(Known::Bool(false)),
                            _ => None,
                        };
                    }
                    self.result(v, span)?;
                }
                Ok(Vec::new())
            }
        }
    }

    /// `pattern_formula`, of MIR pattern `pat`: whether it matches `s`,
    /// exactly, if the logic can say.
    fn pat_formula(&mut self, pat: &Pat, s: &Val) -> Option<Formula> {
        match (pat, s) {
            (Pat::Any, _) => Some(Formula::True),
            (Pat::Range(Some(lo), Some(hi)), Val::Int(l)) if lo == hi => {
                Formula::eq(l, &Lin::constant(*lo))
            }
            (Pat::Range(lo, hi), Val::Int(l)) => Some(Formula::and([
                match lo {
                    Some(lo) => Formula::le(&Lin::constant(*lo), l)?,
                    None => Formula::True,
                },
                match hi {
                    Some(hi) => Formula::le(l, &Lin::constant(*hi))?,
                    None => Formula::True,
                },
            ])),
            (Pat::Bool(b), Val::Bool(f)) => Some(if *b { f.clone() } else { f.clone().not() }),
            (Pat::Or(alts), _) => {
                let mut each = Vec::new();
                for p in alts {
                    each.push(self.pat_formula(p, s)?);
                }
                Some(Formula::or(each))
            }
            (Pat::Variant(v, pats), _) => {
                let (tag, fields) = self.parts_of(s, *v)?;
                let mut all = vec![tag];
                for (p, f) in pats.iter().zip(fields) {
                    all.push(self.pat_formula(p, &f)?);
                }
                Some(Formula::and(all))
            }
            (Pat::Struct(fields), Val::Struct(_, vals)) => {
                let mut all = Vec::new();
                for (j, p) in fields {
                    all.push(self.pat_formula(p, vals.get(*j)?)?);
                }
                Some(Formula::and(all))
            }
            (Pat::Deref(p), _) => {
                let inner = self.pointee(s.clone());
                self.pat_formula(p, &inner)
            }
            _ => None,
        }
    }

    /// `pattern_implies`, of MIR pattern `pat`.
    fn pat_implies(&mut self, pat: &Pat, s: &Val) -> Option<Formula> {
        match (pat, s) {
            (Pat::Variant(v, pats), _) => {
                let (tag, fields) = self.parts_of(s, *v)?;
                let mut all = vec![tag];
                for (p, f) in pats.iter().zip(fields) {
                    all.extend(self.pat_formula(p, &f));
                    all.extend(self.pat_implies(p, &f));
                }
                Some(Formula::and(all))
            }
            (Pat::Struct(fields), Val::Struct(_, vals)) => {
                let mut all = Vec::new();
                for (j, p) in fields {
                    let f = vals.get(*j)?.clone();
                    all.extend(self.pat_formula(p, &f));
                    all.extend(self.pat_implies(p, &f));
                }
                Some(Formula::and(all))
            }
            (Pat::Str(text), _) => {
                if !self.cx.measures.contains_key(STR_LEN) {
                    return None;
                }
                let Val::Int(len) = self.measure(STR_LEN, s.clone()) else {
                    return None;
                };
                let n = i128::try_from(text.len()).expect("a literal fits");
                Formula::eq(&len, &Lin::constant(n))
            }
            (Pat::Or(alts), _) => {
                let each = alts.iter().map(|a| self.pat_implies(a, s));
                Some(Formula::or(each.collect::<Option<Vec<_>>>()?))
            }
            (Pat::Deref(p), _) => {
                let inner = self.pointee(s.clone());
                self.pat_implies(p, &inner)
            }
            _ => None,
        }
    }

    /// That `s`, an enum's value, is variant `v`, and its fields' values
    /// then, if what it is is known.
    fn parts_of(&self, s: &Val, v: usize) -> Option<(Formula, Vec<Val>)> {
        let Val::Obj(o) = s else {
            return None;
        };
        let parts = self.parts.get(o)?;
        let tag = Formula::eq(&parts.tag, &Lin::constant(i128::try_from(v).ok()?))?;
        Some((tag, parts.fields.get(v)?.clone()))
    }

    /// The value of operand `o`, at `span`.
    fn operand(&mut self, fr: &Frame, o: &Operand, span: Span) -> Result<Val, Error> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => Ok(self.read_mir(fr, p)),
            Operand::Const(c) => self.constant(c, span),
        }
    }

    /// The value of constant `c`, used at `span`.
    fn constant(&mut self, c: &Const, span: Span) -> Result<Val, Error> {
        Ok(match c {
            Const::Int(n, _) => Val::Int(Lin::constant(*n)),
            Const::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            Const::Char(c) => Val::Int(Lin::constant(i128::from(u32::from(*c)))),
            Const::Unit | Const::Bytes(_) | Const::Receiver(_) => Val::Other,
            // A string literal: a reference no one else has, as long as its
            // text is.
            Const::Str(text) => {
                let o = self.new_obj();
                let v = Val::Ref(o, Pointee::Unknown);
                if self.cx.measures.contains_key(STR_LEN)
                    && let Ok(n) = i128::try_from(text.len())
                    && let Val::Int(len) = self.measure(STR_LEN, v.clone())
                    && let Some(f) = Formula::eq(&len, &Lin::constant(n))
                {
                    self.fact(f);
                }
                v
            }
            Const::Fn(symbol, _) => {
                self.as_value(symbol, span)?;
                Val::Other
            }
            Const::Param(name) => self.length(&Type::Param(name.clone(), false)),
            Const::Static(_) => Val::Other,
        })
    }

    /// The type of operand `o`.
    fn operand_ty(&self, fr: &Frame, o: &Operand) -> Type {
        match o {
            Operand::Copy(p) | Operand::Move(p) => self.mir_ty(fr, p),
            Operand::Const(c) => match c {
                Const::Int(_, t) => self.cx.solved(t),
                Const::Bool(_) => Type::Bool,
                Const::Char(_) => Type::Char,
                Const::Param(_) => Type::Int(IntTy::Usize),
                _ => Type::Unit,
            },
        }
    }

    /// The type of what's at MIR place `mp`.
    fn mir_ty(&self, fr: &Frame, mp: &crate::mir::Place) -> Type {
        let mut t = self.cx.solved(&fr.body.locals[mp.local.0].ty);
        for p in &mp.proj {
            t = match (p, &t) {
                (crate::mir::Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => {
                    self.cx.solved(inner)
                }
                (crate::mir::Proj::Field(_), Type::Array(elem, _)) => self.cx.solved(elem),
                (crate::mir::Proj::Field(i), t) => match struct_fields(&self.cx.adts, t) {
                    Some(fields) if *i < fields.len() => self.cx.solved(&fields[*i].1),
                    _ => Type::Unit,
                },
                (crate::mir::Proj::Variant(v, j), t) => {
                    let fields = crate::types::variant_fields(&self.cx.adts, t, *v);
                    match fields.get(*j) {
                        Some(ft) => self.cx.solved(ft),
                        None => Type::Unit,
                    }
                }
                _ => Type::Unit,
            };
        }
        t
    }

    /// The place `mp` is, if it's one the walk follows, found without
    /// reading anything: a variable's (or in a closure's body, one it
    /// captured), through fields and `*`s, as `place_of` finds one.
    fn static_place(&self, fr: &Frame, mp: &crate::mir::Place) -> Option<Place> {
        let (mut p, from) = self.root_place(fr, mp)?;
        for proj in &mp.proj[from..] {
            p = match proj {
                crate::mir::Proj::Deref => self.deref(p),
                crate::mir::Proj::Field(i) => {
                    if let Type::Array(..) = self.place_ty(&p) {
                        return None;
                    }
                    p.then(Proj::Field(*i))
                }
                crate::mir::Proj::Variant(..) => return None,
            };
        }
        Some(p)
    }

    /// The variable `mp` starts at, and how many of its projections that
    /// takes: its local's, or in a closure's body, the one at the field of
    /// its closure that captured it.
    fn root_place(&self, fr: &Frame, mp: &crate::mir::Place) -> Option<(Place, usize)> {
        if let Some(id) = fr.id(mp.local) {
            return Some((Place::local(id), 0));
        }
        if fr.env != Some(mp.local) {
            return None;
        }
        let mut k = 0;
        if let Type::Ref(..) = fr.body.locals[mp.local.0].ty {
            if mp.proj.first() != Some(&crate::mir::Proj::Deref) {
                return None;
            }
            k = 1;
        }
        let Some(crate::mir::Proj::Field(i)) = mp.proj.get(k) else {
            return None;
        };
        let var = fr.body.env.get(*i)?;
        k += 1;
        if var.capture != Capture::Value {
            if mp.proj.get(k) != Some(&crate::mir::Proj::Deref) {
                return None;
            }
            k += 1;
        }
        Some((Place::local(var.id), k))
    }

    /// Where the value at MIR place `mp` is (see `Loc`), reading what it
    /// must to find it: an enum's value, for a variant's field, and the
    /// value of a temporary.
    fn locate(&mut self, fr: &Frame, mp: &crate::mir::Place) -> Loc {
        let pinned = (self.pinned.iter().rev())
            .find(|(q, _)| q.local == mp.local && mp.proj.starts_with(&q.proj))
            .map(|(q, v)| (q.clone(), v.clone()));
        let (mut loc, from) = match pinned {
            Some((q, v)) => {
                let t = self.mir_ty(fr, &q);
                (Loc::Val(v, t), q.proj.len())
            }
            None => match self.root_place(fr, mp) {
                Some((p, k)) => (Loc::Place(p), k),
                None => {
                    let v = (self.slots.get(&mp.local)).map_or(Val::Other, |s| s.val.clone());
                    let t = self.cx.solved(&fr.body.locals[mp.local.0].ty);
                    (Loc::Val(v, t), 0)
                }
            },
        };
        for proj in &mp.proj[from..] {
            loc = self.project_loc(loc, *proj);
        }
        loc
    }

    fn project_loc(&mut self, loc: Loc, proj: crate::mir::Proj) -> Loc {
        use crate::mir::Proj as P;
        match (loc, proj) {
            (Loc::Place(q), P::Deref) => Loc::Place(self.deref(q)),
            (Loc::Place(q), P::Field(i)) => match self.place_ty(&q) {
                Type::Array(elem, _) => Loc::Val(Val::Other, self.cx.solved(&elem)),
                _ => Loc::Place(q.then(Proj::Field(i))),
            },
            (Loc::Place(q), P::Variant(v, j)) => {
                let t = self.place_ty(&q);
                let whole = self.read_place(&q);
                self.variant_field(whole, &t, v, j)
            }
            // What a reference or box at no place points to, and what it
            // says of that (as the AST's walk does `*e`).
            (Loc::Val(v, t), P::Deref) => {
                let inner = match &t {
                    Type::Ref(_, _, inner) | Type::Box(inner) => self.cx.solved(inner),
                    _ => Type::Unit,
                };
                let (o, got) = match v {
                    Val::Ref(_, Pointee::Place(q)) => return Loc::Place(q),
                    Val::Ref(o, Pointee::Val(w)) => (Some(o), *w),
                    Val::Ref(o, Pointee::Unknown) => (Some(o), self.unknown(&inner)),
                    _ => (None, self.unknown(&inner)),
                };
                if let Some(o) = o
                    && let Some(tree) = self.trees.get(&o).cloned()
                {
                    self.assume_tree(&got, &inner, &targs::below(&tree, 0));
                }
                Loc::Val(got, inner)
            }
            (Loc::Val(v, t), P::Field(i)) => {
                if let Type::Array(elem, _) = &t {
                    return Loc::Val(Val::Other, self.cx.solved(elem));
                }
                let ft = match struct_fields(&self.cx.adts, &t) {
                    Some(fields) if i < fields.len() => self.cx.solved(&fields[i].1),
                    _ => Type::Unit,
                };
                let vals = match v {
                    Val::Struct(_, vals) => vals,
                    _ => match self.unknown(&t) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                };
                Loc::Val(vals.get(i).cloned().unwrap_or(Val::Other), ft)
            }
            (Loc::Val(v, t), P::Variant(vi, j)) => self.variant_field(v, &t, vi, j),
        }
    }

    /// Field `j` of variant `v` of `whole`, an enum's value of type `t`.
    fn variant_field(&mut self, whole: Val, t: &Type, v: usize, j: usize) -> Loc {
        let fields = crate::types::variant_fields(&self.cx.adts, t, v);
        let ft = fields.get(j).map_or(Type::Unit, |ft| self.cx.solved(ft));
        let val = match whole {
            Val::Obj(o) => (self.parts.get(&o)).and_then(|p| p.fields.get(v)?.get(j).cloned()),
            _ => None,
        };
        Loc::Val(val.unwrap_or(Val::Other), ft)
    }

    /// The value at MIR place `mp`.
    fn read_mir(&mut self, fr: &Frame, mp: &crate::mir::Place) -> Val {
        match self.locate(fr, mp) {
            Loc::Place(p) => self.read_place(&p),
            Loc::Val(Val::Other, t) => self.unknown(&t),
            Loc::Val(v, _) => v,
        }
    }
}
