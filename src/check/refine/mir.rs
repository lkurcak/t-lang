//! The refinement checker's walk over each body's MIR, as the borrow
//! checker built it (docs/65-refinements-on-mir.md, docs/66-one-walk.md):
//! what's known and how it's proved is `Walk`'s (`refine.rs`, `targs`,
//! `logic`); this is how it gets around a body, and finds places:
//!
//! - A variable or parameter is the place of its binding, so predicates
//!   name it by its binding's id; a temporary, or the result, is no place
//!   but a value (`Slot`), whatever borrows or projections of it MIR takes
//!   (`Walk::locate`).
//! - The blocks are walked in reverse postorder, the first of two branches
//!   first. Where blocks meet, what's known is what all of them knew (the
//!   facts they share, from where they parted), and that one of them was
//!   taken (`Walk::join_with`). A branch's condition, or the pattern of a
//!   `match`'s arm and the negation of those before it, is what's known
//!   along its edge.
//! - At a loop's head (`Body::heads`), from the edges into it from
//!   outside: its invariant is proved; what it changes is forgotten, but
//!   for its frame; and its invariant is assumed. Along each edge back to
//!   it, its invariant is proved (lemma I1 of docs/54-loop-invariants.md).
//! - A place changes but by assignment only through a `&mut` of it, a
//!   loan, which the borrow checker lets nothing else read or write the
//!   place while it lives (docs/69-loans.md). A `&mut` given straight to a
//!   call (an operand of it) is lent to the call: what's known of the
//!   place is forgotten once it returns, when its postconditions are
//!   learned. Any other `&mut` forgets it where it's made: an explicit
//!   one, a closure's capture (but for the measures its body keeps, its
//!   frame: `Walk::closure_frames`), or a pattern's binding.
//! - A pattern's binding by `&mut` is a `&mut` of the place it binds, a
//!   variant's field or an array's element too: what's written through it
//!   must have what's declared there (`Walk::bind_ref`).
//! - A closure is walked where it's made, from what's known there, or, if
//!   it's given straight to a function, at the call, after the other
//!   arguments, counting on what they say of its parameters.

use super::*;
use crate::mir::{
    Agg, Bb, Body, Const, Key, Local, LocalKind, LoopNote, Notes, Operand, Pat, Point, Rvalue,
    Statement, StmtKind, StmtNote, Term,
};

/// What a local that's no variable holds: a temporary's value, and, if
/// it's a borrow, `&mut` or not, of a place, which place.
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
    /// Each local that's an operand of a closure's making: the closure,
    /// which captures what it holds.
    captured_by: HashMap<Local, NodeId>,
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
        let mut captured_by = HashMap::new();
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
                    Rvalue::Aggregate(Agg::Closure(l), _, ops) => {
                        for o in ops {
                            if let Some(p) = o.place()
                                && p.proj.is_empty()
                            {
                                captured_by.insert(p.local, *l);
                            }
                        }
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
            captured_by,
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
/// `else`, a `match`'s arms in order), as they're written.
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
        }
    }

    /// The walk, leaving in `failed` the proofs that failed, which don't
    /// stop it (`Walk::failed`), and in `known` the const's value, if it's
    /// one whose value it worked out; giving the operators proved not to
    /// panic, and why each other one wasn't.
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
            unreachable!("the borrow checker keeps the MIR of each body it passes")
        };
        let fr = Frame::new(body, notes);
        let mut w = self.new_walk(f);
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
    /// What's known here, taken out of the walk.
    fn take_state(&mut self) -> State {
        State {
            facts: std::mem::take(&mut self.facts),
            cur: std::mem::take(&mut self.cur),
            objs: std::mem::take(&mut self.objs),
            reachable: self.reachable,
            invalidated: std::mem::take(&mut self.invalidated),
            slots: std::mem::take(&mut self.slots),
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
                self.mir_loop_start(fr, bodies, lp)?;
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

    /// Where paths `states` meet: as `join_with` says, from the facts
    /// they all share, where they parted.
    fn join_states(&mut self, fr: &Frame, mut states: Vec<State>) -> State {
        // What's out of scope on any path is out of scope here, even where
        // only one path gets here.
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
        state
    }

    /// The head of loop `lp`, from outside it (`loop_start`): what it
    /// changes is each place assigned or lent `&mut` in its blocks, and
    /// its frame keeps what the calls it's lent to straight, and the
    /// closures it's captured by, keep.
    fn mir_loop_start(&mut self, fr: &Frame, bodies: &Bodies, lp: &LoopNote) -> Result<(), Error> {
        let (changed, changes) = self.changes_in(fr, bodies, &lp.blocks);
        let frames = self.frames_of(&changes, &changed);
        self.loop_start(&changed, &frames, fr.invariant(lp.head), lp.span)
    }

    /// What `blocks` of body `fr` change: each place assigned or lent
    /// `&mut` in them, with where; and each change, with what it keeps
    /// (`Walk::frames_of`): a `&mut` lent straight to a call, the measures
    /// the call keeps of it (`lent_keeps`); one a closure captures, the
    /// measures its body keeps of each place in it (`closure_frames`);
    /// anything else, nothing.
    fn changes_in(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
        blocks: &[Bb],
    ) -> (Vec<Invalidation>, Vec<(Place, Keeps)>) {
        let mut changed = Vec::new();
        let mut changes = Vec::new();
        for &b in blocks {
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
                    && let Some(p) = self.lent_place(fr, place)
                {
                    changed.push(Invalidation::new(p.clone(), s.span, "assignment"));
                    changes.push((p, Vec::new()));
                }
                let Rvalue::Ref(_, true, q) = value else {
                    continue;
                };
                let Some(p) = self.lent_place(fr, q) else {
                    continue;
                };
                // Lent to a call, by the call.
                let call = (place.proj.is_empty())
                    .then(|| fr.operand_of.get(&place.local))
                    .flatten();
                let span = match call {
                    Some((at, _)) => fr.body.blocks[at.block.0].stmts[at.index].span,
                    None => s.span,
                };
                changed.push(Invalidation::new(p.clone(), span, "mutable lending"));
                let captured = (place.proj.is_empty())
                    .then(|| fr.captured_by.get(&place.local))
                    .flatten();
                let keeps = match captured {
                    // Each place its frame is of changes too, keeping that.
                    Some(&l) => {
                        let kept = self.closure_frames(bodies, l, &p);
                        for (f, _) in &kept {
                            changed.push(Invalidation::new(f.clone(), span, "mutable lending"));
                        }
                        kept
                    }
                    None if place.proj.is_empty()
                        && self.static_place(fr, q) == Some(p.clone()) =>
                    {
                        let kept = self.lent_keeps(fr, place.local, &p);
                        kept.map(|ms| vec![(p.clone(), ms)]).unwrap_or_default()
                    }
                    None => Vec::new(),
                };
                changes.push((p, keeps));
            }
        }
        (changed, changes)
    }

    /// What closure `l`'s calls keep of what's in place `q`, which it
    /// captures by `&mut`: each place in `q` its body changes, with the
    /// measures every change to it in the body keeps (`frames_of` over its
    /// blocks, as a loop's), which, as nothing else can change `q` while
    /// the closure has it, its loan keeps (lemma F2 of docs/69-loans.md).
    fn closure_frames(&mut self, bodies: &Bodies, l: NodeId, q: &Place) -> Keeps {
        let key = Key::Closure(l);
        let Some((_, body, notes)) = bodies.iter().find(|(k, ..)| *k == key) else {
            return Vec::new();
        };
        let fr = Frame::new(body, notes);
        let blocks = fr.rpo.clone();
        let (changed, changes) = self.changes_in(&fr, bodies, &blocks);
        let frames = self.frames_of(&changes, &changed);
        frames.into_iter().filter(|(f, _)| f.within(q)).collect()
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
        if let Some(dropped) = fr.notes.drops.get(&at) {
            for p in dropped {
                self.mir_drop(fr, p, s.span);
            }
        }
        match &s.kind {
            StmtKind::Assign {
                place,
                value,
                declare,
            } => self.mir_assign(fr, bodies, at, s.span, place, value, *declare),
            StmtKind::Dead(l) => self.mir_dead(fr, *l, s.span),
            StmtKind::Inspect(_)
            | StmtKind::Consume(..)
            | StmtKind::Refine(..)
            | StmtKind::Holds(_)
            | StmtKind::Kept(_)
            | StmtKind::Drop(..)
            | StmtKind::Free(_) => Ok(()),
        }
    }

    /// The value at `place` may be dropped here (`Notes::drops`): if its
    /// type's drop runs a `discard` that can write where the type is
    /// invariant (`targs::drop_changes`), nothing may be declared there, as
    /// that `discard` is a call it's given (lemma A9 of
    /// docs/68-fail-closed.md).
    fn mir_drop(&mut self, fr: &Frame, place: &crate::mir::Place, span: Span) {
        let ty = self.mir_ty(fr, place);
        let Some(why) = targs::drop_changes(self.cx, &ty) else {
            return;
        };
        let (v, decl, _) = self.lvalue_mir(fr, place);
        let mut tree = self.written_tree(&v, &ty);
        tree.extend(decl);
        for (pos, d) in &tree {
            if d.is_any() {
                continue;
            }
            let Some((pt, crate::borrowck::regions::Variance::Inv)) = self.followed(&ty, pos, span)
            else {
                continue;
            };
            // Anything may be written there: unless what's declared holds
            // of anything here, as of a variant the value isn't.
            if self
                .implies(&[(Vec::new(), targs::Cand::Any)], d, &pt)
                .is_ok()
            {
                continue;
            }
            let text = d.text().unwrap_or_default();
            let what = match (place.proj.is_empty(), fr.id(place.local)) {
                (true, Some(id)) => match self.cx.typed.binds.get(&id) {
                    Some(b) => format!("`{}`", b.name),
                    None => "this value".to_string(),
                },
                _ => "this value".to_string(),
            };
            self.failed(Error::ty(
                span,
                format!(
                    "can't prove `{text}` of what dropping {what} may write: {why} can change a \
                     value of it"
                ),
            ));
            return;
        }
    }

    /// Local `l` goes out of scope, or, if it's declared with no value, is
    /// declared (`let x;`), bound to none.
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
        // A variable's first value, by a `let` or a pattern.
        if declare
            && place.proj.is_empty()
            && let Some(id) = fr.id(place.local)
        {
            return self.mir_bind(fr, span, place.local, id, value);
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

    /// A variable's first value, `value`, by a `let` or a pattern: what
    /// its refinement needs, and what it declares.
    fn mir_bind(
        &mut self,
        fr: &Frame,
        span: Span,
        l: Local,
        id: NodeId,
        value: &Rvalue,
    ) -> Result<(), Error> {
        if let Rvalue::Ref(_, m, q) = value {
            return self.bind_ref(fr, span, id, *m, q);
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
    /// pattern (match ergonomics).
    fn bind_ref(
        &mut self,
        fr: &Frame,
        span: Span,
        id: NodeId,
        m: bool,
        q: &crate::mir::Place,
    ) -> Result<(), Error> {
        let name = (self.cx.typed.binds.get(&id)).map_or(String::new(), |b| b.name.clone());
        // As `&mut` of its place: refused where a write through it would
        // need what's around it proved again (lemma V1 of
        // docs/66-one-walk.md).
        if m && let Some(why) = self.mut_refused(fr, q, "", false) {
            return Err(Error::ty(
                span,
                format!(
                    "can't bind `{name}` by `&mut`: {why}, which a write through the reference \
                     could break"
                ),
            ));
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
            // what's declared at its place (lemma V1 of
            // docs/66-one-walk.md), a variant's field's or an array's
            // element's too (`lvalue_mir`).
            (true, loc) => {
                let (_, decl, _) = self.lvalue_mir(fr, q);
                let o = self.new_obj();
                let tree = targs::under(&decl, 0);
                if !tree.is_empty() {
                    self.trees.insert(o, tree);
                }
                let to = match loc {
                    Loc::Place(p) => Pointee::Place(p),
                    Loc::Val(..) => Pointee::Unknown,
                };
                let r = Val::Ref(o, to);
                widen(self, &r);
                // A loan of the place, which writes through it change:
                // forgotten here, as a `&mut` of it is (lemma S2 of
                // docs/69-loans.md), or the place it's in, if the walk
                // can't name it (S4).
                if let Some(p) = self.lent_place(fr, q) {
                    self.invalidate(&p, span, "mutable lending");
                    self.kill(&p);
                }
                self.bind(id, Some(r));
            }
            // A `&` to a value at no place, a variant's or an array's
            // element's: what's written through it (in a `Cell`) must be
            // what's declared there, as for a `&mut` (lemma C1 of
            // docs/68-fail-closed.md).
            (false, Loc::Val(v, _)) => {
                let (_, decl, _) = self.lvalue_mir(fr, q);
                let o = self.new_obj();
                let tree = targs::under(&decl, 0);
                if !tree.is_empty() {
                    self.trees.insert(o, tree);
                }
                let to = match v {
                    Val::Other => Pointee::Unknown,
                    v => Pointee::Val(Box::new(v)),
                };
                let r = Val::Ref(o, to);
                widen(self, &r);
                self.bind(id, Some(r));
            }
        }
        Ok(())
    }

    /// An assignment of `new` to `place`, which isn't a temporary's: what's
    /// written must have what's declared there, and keep the refinements of
    /// what it's in.
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
        self.keep_wholes(fr, place, &new, &text, vspan)?;
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
            // `-n`: the same number, which must fit: the `-` is proved not
            // to overflow.
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
            Rvalue::Ref(_, m, q) => self.mir_borrow(fr, bodies, at, span, dest, *m, q),
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
                self.need_variant(ty, v, &vals, &spans, span)?;
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
                let spans: Vec<Span> = (0..vals.len()).map(span_of).collect();
                self.elements(&v, ty, &vals, &spans, span);
                Ok(v)
            }
            Agg::Repeat => {
                let v = self.unknown(ty);
                self.elements(&v, ty, &vals, &[], span);
                Ok(v)
            }
            Agg::Closure(_) => unreachable!("walked above"),
        }
    }

    /// Walks closure `l`, of type `ty`, from what's known here: then goes
    /// back to what's known here, as making one runs nothing.
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

    /// A `&` or `&mut` (`m`) of `q`, made into `dest` (`borrow_mir`): if
    /// it's a `&mut` not given straight to a call, what's known of the
    /// place is forgotten here, but for what a closure capturing it keeps
    /// (its frame), and if it is, once the call returns.
    #[allow(clippy::too_many_arguments)]
    fn mir_borrow(
        &mut self,
        fr: &Frame,
        bodies: &Bodies,
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
        // Lent to a call, it's the call that forgets it, after; but if the
        // walk can't name it, only a place it's in, it's forgotten here.
        let by_call = lent_to.is_some() && self.static_place(fr, q).is_some();
        if m && !by_call
            && let Some(p) = self.lent_place(fr, q)
        {
            let frames = match dest.proj.is_empty() {
                true => match fr.captured_by.get(&dest.local) {
                    Some(&l) => self.closure_frames(bodies, l, &p),
                    None => Vec::new(),
                },
                false => Vec::new(),
            };
            let kept = self.measures_now(&frames);
            self.invalidate(&p, span, "mutable lending");
            self.kill(&p);
            self.measures_kept(kept);
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

    /// Fails if `q`, written `text`, borrowed `&mut` at `span`, can't be
    /// (`mut_refused`).
    fn no_mut_borrow_mir(
        &mut self,
        fr: &Frame,
        q: &crate::mir::Place,
        text: &str,
        span: Span,
        scoped: bool,
    ) -> Result<(), Error> {
        match self.mut_refused(fr, q, text, scoped) {
            Some(why) => Err(Error::ty(
                span,
                format!(
                    "can't borrow `{text}` as `&mut`: {why}, which a write through the reference \
                     could break"
                ),
            )),
            None => Ok(()),
        }
    }

    /// Why `q`, written `text` (or nothing, if it's a pattern's), can't be
    /// borrowed `&mut`, if it can't: a refined local, or a field a
    /// refinement is about, unless it's lent straight to a call-scoped
    /// parameter (`scoped`): then the refinements are proved again when
    /// the call returns (`post_now`), and until then nothing can read the
    /// struct, which the borrow is of a part of; or a part of a value
    /// whose refinement is of the whole of it, a refined local's or what's
    /// declared of one (`wholes_around`), which nothing proves again of
    /// the whole when a part changes through the reference.
    fn mut_refused(
        &mut self,
        fr: &Frame,
        q: &crate::mir::Place,
        text: &str,
        scoped: bool,
    ) -> Option<String> {
        let place = self.static_place(fr, q);
        if let Some(p) = &place
            && p.path.is_empty()
            && self.invariants.contains_key(&p.root)
            && !scoped
        {
            return Some("its type has a refinement".to_string());
        }
        // Nor a part of one, through fields and boxes: a write to the part
        // is a write to it.
        if let Some(p) = &place
            && !p.path.is_empty()
            && let Some(name) = self.in_refined_place(p)
        {
            return Some(format!("it's in `{name}`, whose type has a refinement"));
        }
        let why = match (scoped, &place) {
            (false, _) => self.constrained_mir(fr, q, text, false),
            (true, None) => self.constrained_mir(fr, q, text, true),
            (true, Some(_)) => None,
        };
        if why.is_some() {
            return why;
        }
        if let Some(why) = self.way_into(fr, q) {
            return Some(why);
        }
        if let Some(why) = self.variant_constrained(fr, q) {
            return Some(why);
        }
        let (e, n, _) = self.wholes_around(fr, q).into_iter().next()?;
        let name = self.level_text(fr, &e, text, n);
        Some(match name {
            Some(name) => format!("it's in `{name}`, whose type has a refinement"),
            None => "what it's in has a refinement".to_string(),
        })
    }

    /// If `q` is a variant's field, or in one, through fields and boxes,
    /// whose refinement names another field or is named by another's, why
    /// it can't be borrowed `&mut`: what's written through the reference
    /// would change what that refinement is about, which the reference
    /// can't say (as `RefinedFields::constrained` of a struct's), and
    /// nothing proves it again after (docs/67-refinement-aliases.md, V3).
    fn variant_constrained(&self, fr: &Frame, q: &crate::mir::Place) -> Option<String> {
        use crate::mir::Proj as P;
        let mut k = q.proj.len();
        while k > 0 {
            let e = crate::mir::Place {
                local: q.local,
                proj: q.proj[..k - 1].to_vec(),
            };
            match q.proj[k - 1] {
                P::Variant(v, j) => {
                    let t = self.mir_ty(fr, &e);
                    if let Some(rv) = self.refined_variants(&t)
                        && let Some(variant) = rv.variants.get(v)
                        && let Some(why) = variant.constrained(&rv.name, j)
                    {
                        return Some(why);
                    }
                }
                P::Field(_) => {}
                P::Deref if matches!(self.mir_ty(fr, &e), Type::Box(_)) => {}
                P::Deref => return None,
            }
            k -= 1;
        }
        None
    }

    /// If `q` is in a variant's field (as a pattern binds one) of an enum
    /// that's a struct's field, whose own refinement says something of
    /// what's there (by a way into it, `Part::ways`), why it can't be
    /// borrowed `&mut`: as the field itself can't be (`constrained_mir`),
    /// as that refinement is of the struct, in the logic, which a write
    /// through the reference would change. Through fields and boxes, not
    /// references, as `constrained_mir`.
    fn way_into(&mut self, fr: &Frame, q: &crate::mir::Place) -> Option<String> {
        use crate::mir::Proj as P;
        let mut k = q.proj.len();
        while k > 0 {
            match q.proj[k - 1] {
                P::Variant(..) => {
                    let end = k;
                    while k > 0 && matches!(q.proj[k - 1], P::Variant(..)) {
                        k -= 1;
                    }
                    let way: Way = (q.proj[k..end].iter())
                        .filter_map(|p| match p {
                            P::Variant(v, j) => Some((*v, *j)),
                            _ => None,
                        })
                        .collect();
                    if k > 0
                        && let P::Field(i) = q.proj[k - 1]
                    {
                        let st = crate::mir::Place {
                            local: q.local,
                            proj: q.proj[..k - 1].to_vec(),
                        };
                        let t = self.mir_ty(fr, &st);
                        // Unless it's about the field alone, which a
                        // reference into it says (`targs::field_decl`).
                        if let Some(f) = self.refined_fields(&t)
                            && let Some(Some(r)) = f.refs.get(i)
                            && !f.alone(i)
                            && (r.parts.iter())
                                .any(|p| !p.carried && p.ways.iter().any(|w| w.starts_with(&way)))
                        {
                            return Some(f.why(i, i));
                        }
                    }
                }
                P::Deref => {
                    let before = crate::mir::Place {
                        local: q.local,
                        proj: q.proj[..k - 1].to_vec(),
                    };
                    if !matches!(self.mir_ty(fr, &before), Type::Box(_)) {
                        return None;
                    }
                    k -= 1;
                }
                P::Field(_) => k -= 1,
            }
        }
        None
    }

    /// The values `q` is in that a write to it changes, from the innermost
    /// out, through fields and boxes, up to what a reference points to or a
    /// variant's field is (what an enum's refinement says is only which
    /// variant it is, by its measures), with what's declared of the whole
    /// of each, of those it says something of: each as its place, how many
    /// fields out from `q` it is, and that.
    fn wholes_around(
        &mut self,
        fr: &Frame,
        q: &crate::mir::Place,
    ) -> Vec<(crate::mir::Place, usize, targs::Tree)> {
        let mut out = Vec::new();
        let mut n = 0;
        for (e, step) in self.levels(fr, q) {
            if let crate::mir::Proj::Field(_) = step {
                n += 1;
            }
            let (_, decl, _) = self.lvalue_mir(fr, &e);
            let whole: targs::Tree = (decl.into_iter())
                .filter(|(pos, d)| pos.is_empty() && !d.is_any())
                .collect();
            if !whole.is_empty() {
                out.push((e, n, whole));
            }
        }
        out
    }

    /// The values `q` is in that a write to it changes (`wholes_around`),
    /// innermost first, each with the step from it in.
    fn levels(
        &self,
        fr: &Frame,
        q: &crate::mir::Place,
    ) -> Vec<(crate::mir::Place, crate::mir::Proj)> {
        let mut out = Vec::new();
        for k in (1..=q.proj.len()).rev() {
            let e = crate::mir::Place {
                local: q.local,
                proj: q.proj[..k - 1].to_vec(),
            };
            let step = q.proj[k - 1];
            let on = match step {
                crate::mir::Proj::Field(_) => true,
                crate::mir::Proj::Deref => matches!(self.mir_ty(fr, &e), Type::Box(_)),
                crate::mir::Proj::Variant(..) => false,
            };
            if !on {
                break;
            }
            out.push((e, step));
        }
        out
    }

    /// The text of `e`, `n` fields out from a place written `text`: from
    /// that text, if there's one, or else its place's name, if it has one.
    fn level_text(
        &self,
        fr: &Frame,
        e: &crate::mir::Place,
        text: &str,
        n: usize,
    ) -> Option<String> {
        if !text.is_empty() {
            return Some(outer_text(text, n).to_string());
        }
        let p = self.static_place(fr, e)?;
        self.place_name(&p)
    }

    /// Before `new` is written to `place`, written `text`, at `span`: each
    /// value it's in that the write changes, up to what a reference points
    /// to (`levels`), must have what's declared of the whole of it after,
    /// as it is with the part written replaced. (What's declared of the
    /// part itself, `keep_declared` and `need_tree` check; a refined
    /// local's own refinement, `keep_invariant`.)
    fn keep_wholes(
        &mut self,
        fr: &Frame,
        place: &crate::mir::Place,
        new: &Val,
        text: &str,
        span: Span,
    ) -> Result<(), Error> {
        let levels = self.levels(fr, place);
        if levels.is_empty() || self.wholes_around(fr, place).is_empty() {
            return Ok(());
        }
        let mut part = new.clone();
        let mut n = 0;
        for (e, step) in levels {
            let (v, decl, t) = self.lvalue_mir(fr, &e);
            let whole: targs::Tree = (decl.into_iter())
                .filter(|(pos, d)| pos.is_empty() && !d.is_any())
                .collect();
            part = match step {
                crate::mir::Proj::Field(i) => {
                    n += 1;
                    let mut vals = match v {
                        Val::Struct(_, vals) => vals,
                        _ => match self.unknown(&t) {
                            Val::Struct(_, vals) => vals,
                            _ => Vec::new(),
                        },
                    };
                    match vals.get_mut(i) {
                        Some(f) => {
                            *f = part;
                            Val::Struct(self.new_obj(), vals)
                        }
                        // An array's element: what's in it isn't followed.
                        None => Val::Other,
                    }
                }
                _ => Val::Ref(self.new_obj(), Pointee::Val(Box::new(part))),
            };
            if !whole.is_empty() {
                let what = match self.level_text(fr, &e, text, n) {
                    Some(name) if !text.is_empty() => format!("`{name}`, once `{text}` is written"),
                    _ => "the value it's written in".to_string(),
                };
                self.need_tree(&part, &t, &whole, None, span, &what)?;
            }
        }
        Ok(())
    }

    /// The name of the refined local `p` is a part of, through fields and
    /// boxes, if it's one: not the local itself, nor through a reference,
    /// as what's behind one isn't part of its value (and no refinement of a
    /// `mut` variable names what's behind a `&mut`).
    fn in_refined_place(&mut self, p: &Place) -> Option<String> {
        if !self.boxes_only(p) {
            return None;
        }
        self.invariants.get(&p.root).map(|(_, name)| name.clone())
    }

    /// If `q`, written `text`, is a field a refinement is about, why it
    /// can't be borrowed `&mut`.
    ///
    /// Or a place in such a field, through fields, variants' fields and
    /// boxes: a write to it changes the field. Not through a reference:
    /// what's behind one isn't part of the value, and no refinement can
    /// name what's behind a `&mut`, nor anything write through a `&`: only
    /// a cell's `replace` does, and no refinement can name what's in a
    /// cell (lemma K1 of docs/37-cell.md). `unplaced`: of a struct at no
    /// place, lent to a call (`RefinedFields::constrained_unplaced`).
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
                        false => f.refused(i),
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

    /// A `&` or `&mut` of `q`: a reference that says what's declared of
    /// the value it points to, which each write through it must keep
    /// (lemma W1 of docs/63-what-is-declared.md). A reborrow of what a
    /// reference points to is that reference, the same object, saying what
    /// it says; one of a place says what `place_decl` says; one of a value
    /// at no place (what a reference or a box a call gave points to, a
    /// field of that), what `lvalue_mir` says.
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

    /// The value at `q`, what's declared of it there, and its type: what
    /// each write there must keep (lemma W1 of
    /// docs/63-what-is-declared.md). By cases on what `q` is, each case
    /// saying it: a place a local is the root of, `place_decl`; what a
    /// reference or a box points to, what's declared below the top of what
    /// it's in, and what the reference says (`deref_lvalue`); a field,
    /// what's declared of what it's in at the field's positions
    /// (`field_decl`), and a variant's field, at that variant's field's
    /// (`variant_decl`); an array's element, what's declared of its
    /// elements; and a temporary nothing else can name, what a `let` of it
    /// would declare: what its value's type says (`widen`).
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
            // An array's element (from a pattern): what's declared of its
            // elements.
            crate::mir::Proj::Field(_) if matches!(self.mir_ty(fr, &outer), Type::Array(..)) => {
                let (_, decl, _) = self.lvalue_mir(fr, &outer);
                let v = self.read_mir(fr, q);
                (v, targs::below(&decl, 0), t)
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
            crate::mir::Proj::Variant(v, j) => {
                let (_, decl, et) = self.lvalue_mir(fr, &outer);
                let decl = self.variant_decl(&decl, &et, *v, *j);
                let v = self.read_mir(fr, q);
                (v, decl, t)
            }
        }
    }

    /// A call of `f` with `ops`, into `dest` (`call_at`): its arguments'
    /// refinements needed, what's lent to it forgotten, its result and
    /// postconditions learned.
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

    /// Whether pattern `pat` matches `s`, exactly, if the logic can say:
    /// so it doesn't for the arms after one that has it.
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

    /// What it says of `s` that `pat` matches it, beyond `pat_formula`
    /// (which is exactly when it matches): a string pattern of `n` bytes
    /// matches only a `&str` whose `len()` is `n`, but not every such
    /// one. So this may be assumed where `pat` matched, and its negation
    /// never where it didn't (lemma S1 of docs/42-patterns-and-loops.md).
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
    /// captured), through fields and `*`s.
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

    /// The place a loan of `mp` can change, as the walk names places: the
    /// place it is (`static_place`), or, if it's in a variant's field or an
    /// array's element, which the walk doesn't name, the place that's in
    /// (lemma S4 of docs/69-loans.md).
    fn lent_place(&self, fr: &Frame, mp: &crate::mir::Place) -> Option<Place> {
        let (mut p, from) = self.root_place(fr, mp)?;
        for proj in &mp.proj[from..] {
            p = match proj {
                crate::mir::Proj::Deref => self.deref(p),
                crate::mir::Proj::Field(i) => {
                    if let Type::Array(..) = self.place_ty(&p) {
                        return Some(p);
                    }
                    p.then(Proj::Field(*i))
                }
                crate::mir::Proj::Variant(..) => return Some(p),
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
        let (mut loc, from) = match self.root_place(fr, mp) {
            Some((p, k)) => (Loc::Place(p), k),
            None => {
                let v = (self.slots.get(&mp.local)).map_or(Val::Other, |s| s.val.clone());
                let t = self.cx.solved(&fr.body.locals[mp.local.0].ty);
                (Loc::Val(v, t), 0)
            }
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
            // says of that.
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
