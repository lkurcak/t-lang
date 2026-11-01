//! Drop elaboration: working out, before the program runs, what each
//! `dead`, assignment and `consume` drops (docs/32.1-drops.md).
//!
//! The MIR the borrow checker built says *where* values go away: `dead _3`
//! where `_3` goes out of scope, an assignment where the old value of its
//! place does, a `consume` where a pattern took a value apart. It doesn't
//! say *what's there* to drop then: that depends on the path taken, as a
//! value moved out on one branch isn't on the other. Running MIR could
//! look; a compiled program can't. So this works it out, as rustc's drop
//! elaboration does:
//!
//! - The *move paths* are the places the body owns that something moves
//!   out of, gives a value to, or takes apart, and the places they're in:
//!   locals, their fields, variants' fields, an array's values, what a box
//!   points to. Never what's behind a reference, which the body doesn't own.
//! - A forward dataflow finds, at each point, which move paths *may* hold a
//!   value and which *may not*. A place that's definitely there is dropped
//!   whole, `drop p`; one that's definitely not, not at all.
//! - One that may or may not be there gets a *drop flag*, a `bool` local
//!   set where it gets a value and cleared where it's moved out (with all
//!   of what's in it), and its drop happens only if the flag says so.
//! - A place some of which was moved out is dropped *open*: what's left of
//!   it, part by part, in the order its fields are declared, as dropping it
//!   whole would drop what isn't there. An enum's are by which variant it
//!   is, a box's are what it points to and then `free`. A type with a
//!   `discard` of its own has it called only if all of it is there, as
//!   before: taken apart, nothing calls it.
//!
//! **Invariant (E1).** At every point, a move path's flag is `true` exactly
//! when the value at it is all there but for what's moved out of the move
//! paths inside it: it's set where the path, or what it's in, gets a value,
//! and cleared where it, or what it's in, is moved out, goes out of scope,
//! or is taken apart. So a drop guarded by flags drops exactly what's
//! there. The interpreter still keeps what's in each slot, and checks it:
//! a `drop` of what isn't all there, or a `dead`, assignment or `consume`
//! that leaves behind what needed dropping, is a bug here.
//!
//! It runs on each body once the borrow checker has checked it, at the
//! body's own generic types, on the copy that's kept for running: so the
//! borrow checker's messages never see it, and what it decides depends
//! only on what's moved, which the borrow checker decided once for every
//! copy (S3 of docs/32-mir.md). A place whose type is `Copy` is never
//! dropped: no copy of the body can have anything there to drop.
//! Monomorphisation then leaves out the drops that do nothing at the
//! copy's types.

use super::build::{Builder, project};
use crate::mir::*;
use crate::types::{AdtKind, Type};
use std::collections::{BTreeMap, BTreeSet};

/// The move paths of a body.
struct Paths {
    places: Vec<Place>,
    index: BTreeMap<Place, usize>,
    /// Each path's own, and every path inside it, itself first.
    subtree: Vec<Vec<usize>>,
}

/// What may and may not be at each move path, by index: `init`, it may hold
/// a value; `uninit`, it may not.
#[derive(Clone, PartialEq)]
struct State {
    init: Vec<bool>,
    uninit: Vec<bool>,
}

impl State {
    fn join(&mut self, other: &State) {
        for (a, b) in self.init.iter_mut().zip(&other.init) {
            *a |= b;
        }
        for (a, b) in self.uninit.iter_mut().zip(&other.uninit) {
            *a |= b;
        }
    }
}

/// Elaborates the drops of `body`, the erased copy of the body `b` built
/// and checked, in place.
pub(crate) fn elaborate(b: &Builder, body: &mut Body) {
    let mut e = Elaborator::new(b, body);
    // Once to find which move paths need flags, then for real.
    let _ = e.run(body);
    let flagged = std::mem::take(&mut e.wanted);
    e.flags = BTreeMap::new();
    for m in flagged {
        let l = Local(body.locals.len() + e.flags.len());
        e.flags.insert(m, l);
    }
    let (blocks, locals) = e.run(body);
    body.locals.extend(locals);
    body.blocks = blocks;
    e.initialise_flags(body);
}

struct Elaborator<'b, 'a> {
    b: &'b Builder<'a>,
    locals: Vec<LocalDecl>,
    paths: Paths,
    /// What's at each move path at the start of each block, `None` for one
    /// control never gets to.
    ins: Vec<Option<State>>,
    /// The flag of each move path that has one, by path.
    flags: BTreeMap<usize, Local>,
    /// The move paths found to need flags.
    wanted: BTreeSet<usize>,
    /// Is this a `discard` of a type's own: then its `self`, local 1, is
    /// dropped part by part, as that's what's running (lemma D3 of
    /// docs/20-destructors.md).
    own_discard: bool,
}

/// The blocks being written: the body's, each first part at its index, and
/// those added after.
struct Out {
    blocks: Vec<Block>,
    cur: usize,
    /// Locals added: temporaries for a call's result.
    temps: Vec<LocalDecl>,
    first_temp: usize,
}

impl Out {
    fn push(&mut self, kind: StmtKind, span: Span) {
        self.blocks[self.cur].stmts.push(Statement { kind, span });
    }

    fn new_block(&mut self) -> usize {
        self.blocks.push(Block::default());
        self.blocks.len() - 1
    }

    fn terminate(&mut self, t: Term) {
        self.blocks[self.cur].term = Some(t);
    }
}

use crate::error::Span;

impl<'b, 'a> Elaborator<'b, 'a> {
    fn new(b: &'b Builder<'a>, body: &Body) -> Self {
        let own_discard = b.cx.in_discard
            && !b.closure
            && body.locals.get(1).and_then(|l| l.name.as_deref()) == Some("self");
        let mut e = Elaborator {
            b,
            locals: body.locals.clone(),
            paths: Paths {
                places: Vec::new(),
                index: BTreeMap::new(),
                subtree: Vec::new(),
            },
            ins: Vec::new(),
            flags: BTreeMap::new(),
            wanted: BTreeSet::new(),
            own_discard,
        };
        e.find_paths(body);
        e.ins = e.dataflow(body);
        e
    }

    // Types.

    fn place_ty(&self, place: &Place) -> Type {
        let mut t = self.locals[place.local.0].ty.clone();
        for p in &place.proj {
            t = project(self.b.adts(), &t, *p);
        }
        t
    }

    /// Is `place` behind a reference, which the body doesn't own?
    fn behind(&self, place: &Place) -> bool {
        let mut t = self.locals[place.local.0].ty.clone();
        for p in &place.proj {
            if let (Proj::Deref, Type::Ref(..)) = (p, &t) {
                return true;
            }
            t = project(self.b.adts(), &t, *p);
        }
        false
    }

    fn is_copy(&self, t: &Type) -> bool {
        self.b.cx.is_copy(t)
    }

    // Move paths.

    fn find_paths(&mut self, body: &Body) {
        let mut set: BTreeSet<Place> = (0..body.locals.len())
            .map(|l| Place::local(Local(l)))
            .collect();
        let add = |set: &mut BTreeSet<Place>, p: &Place| {
            for n in 1..=p.proj.len() {
                set.insert(Place {
                    local: p.local,
                    proj: p.proj[..n].to_vec(),
                });
            }
        };
        for s in body.blocks.iter().flat_map(|b| &b.stmts) {
            match &s.kind {
                StmtKind::Assign { place, value, .. } => {
                    for o in value.operands() {
                        if let Operand::Move(p) = o {
                            add(&mut set, p);
                        }
                    }
                    if !self.behind(place) {
                        add(&mut set, place);
                    }
                }
                StmtKind::Consume(p, _) => add(&mut set, p),
                _ => {}
            }
        }
        let places: Vec<Place> = set.into_iter().collect();
        let index: BTreeMap<Place, usize> =
            (places.iter().cloned().enumerate()).map(|(i, p)| (p, i)).collect();
        // In order, what's inside a place comes right after it.
        let subtree = (0..places.len())
            .map(|i| {
                (i..places.len())
                    .take_while(|j| places[*j].within(&places[i]))
                    .collect()
            })
            .collect();
        self.paths = Paths {
            places,
            index,
            subtree,
        };
    }

    fn path(&self, p: &Place) -> usize {
        self.paths.index[p]
    }

    fn set(&self, st: &mut State, m: usize, init: bool) {
        for &d in &self.paths.subtree[m] {
            st.init[d] = init;
            st.uninit[d] = !init;
        }
    }

    /// What statement `s` does to `st`: moves first, then what it gives a
    /// value to.
    fn effect(&self, st: &mut State, s: &StmtKind) {
        match s {
            StmtKind::Assign { place, value, .. } => {
                self.moves(st, value);
                if !self.behind(place) {
                    self.set(st, self.path(place), true);
                }
            }
            StmtKind::Dead(l) => self.set(st, self.path(&Place::local(*l)), false),
            StmtKind::Consume(p, _) => self.set(st, self.path(p), false),
            _ => {}
        }
    }

    fn moves(&self, st: &mut State, value: &Rvalue) {
        for o in value.operands() {
            if let Operand::Move(p) = o {
                self.set(st, self.path(p), false);
            }
        }
    }

    fn entry(&self, body: &Body) -> State {
        let n = self.paths.places.len();
        let mut st = State {
            init: vec![false; n],
            uninit: vec![true; n],
        };
        for (i, l) in body.locals.iter().enumerate() {
            if matches!(l.kind, LocalKind::Param | LocalKind::Env) {
                self.set(&mut st, self.path(&Place::local(Local(i))), true);
            }
        }
        st
    }

    fn dataflow(&self, body: &Body) -> Vec<Option<State>> {
        let mut ins: Vec<Option<State>> = vec![None; body.blocks.len()];
        ins[0] = Some(self.entry(body));
        let mut todo = vec![0];
        while let Some(bb) = todo.pop() {
            let mut st = ins[bb].clone().expect("reached");
            for s in &body.blocks[bb].stmts {
                self.effect(&mut st, &s.kind);
            }
            for succ in body.successors(Bb(bb)) {
                let new = match &ins[succ.0] {
                    None => st.clone(),
                    Some(old) => {
                        let mut new = old.clone();
                        new.join(&st);
                        new
                    }
                };
                if ins[succ.0].as_ref() != Some(&new) {
                    ins[succ.0] = Some(new);
                    todo.push(succ.0);
                }
            }
        }
        ins
    }

    // Writing the elaborated body.

    /// The body's blocks with the drops elaborated, and the locals added.
    fn run(&mut self, body: &Body) -> (Vec<Block>, Vec<LocalDecl>) {
        let mut out = Out {
            blocks: vec![Block::default(); body.blocks.len()],
            cur: 0,
            temps: Vec::new(),
            first_temp: body.locals.len() + self.flags.len(),
        };
        for (bb, block) in body.blocks.iter().enumerate() {
            out.cur = bb;
            let Some(mut st) = self.ins[bb].clone() else {
                out.blocks[bb] = block.clone();
                continue;
            };
            for s in &block.stmts {
                self.statement(&mut out, &mut st, s);
            }
            if let Some(Term::Return) = &block.term {
                for &l in self.flags.values() {
                    out.push(StmtKind::Dead(l), Span::new(0, 0));
                }
            }
            out.blocks[out.cur].term = block.term.clone();
        }
        let mut locals: Vec<LocalDecl> = (self.flags.values())
            .map(|_| LocalDecl {
                ty: Type::Bool,
                kind: LocalKind::Temp,
                mutable: true,
                name: None,
                span: Span::new(0, 0),
                mut_span: None,
                by_ref: None,
                deferred: false,
            })
            .collect();
        locals.extend(out.temps);
        (out.blocks, locals)
    }

    fn statement(&mut self, out: &mut Out, st: &mut State, s: &Statement) {
        let span = s.span;
        match &s.kind {
            StmtKind::Assign {
                place,
                value,
                declare,
            } => {
                let mut after = st.clone();
                self.moves(&mut after, value);
                let behind = self.behind(place);
                let drops = !self.is_copy(&self.place_ty(place))
                    && (behind || self.maybe_there(&after, self.path(place)));
                if drops && matches!(value, Rvalue::Call(..)) {
                    // The call may do what can be seen, which comes before
                    // the old value is dropped: its result waits in a
                    // temporary.
                    let ty = self.place_ty(place);
                    let tmp = Local(out.first_temp + out.temps.len());
                    out.temps.push(LocalDecl {
                        ty,
                        kind: LocalKind::Temp,
                        mutable: true,
                        name: None,
                        span,
                        mut_span: None,
                        by_ref: None,
                        deferred: false,
                    });
                    let kind = StmtKind::Assign {
                        place: Place::local(tmp),
                        value: value.clone(),
                        declare: false,
                    };
                    out.push(kind, span);
                    self.flag_moves(out, value, span);
                    self.drop_at(out, &after, place, behind, span);
                    let kind = StmtKind::Assign {
                        place: place.clone(),
                        value: Rvalue::Use(Operand::Move(Place::local(tmp))),
                        declare: *declare,
                    };
                    out.push(kind, span);
                    out.push(StmtKind::Dead(tmp), span);
                } else {
                    if drops {
                        self.drop_at(out, &after, place, behind, span);
                    }
                    out.push(s.kind.clone(), span);
                    self.flag_moves(out, value, span);
                }
                if !behind {
                    self.flag_set(out, self.path(place), true, span);
                }
            }
            StmtKind::Dead(l) => {
                let m = self.path(&Place::local(*l));
                self.drop_path(out, st, m, span);
                out.push(s.kind.clone(), span);
                self.flag_set(out, m, false, span);
            }
            StmtKind::Consume(p, variant) => {
                let m = self.path(p);
                self.open(out, st, m, *variant, span);
                out.push(s.kind.clone(), span);
                self.flag_set(out, m, false, span);
            }
            _ => out.push(s.kind.clone(), span),
        }
        self.effect(st, &s.kind);
    }

    /// Drops the old value of `place`, which an assignment is about to
    /// give a new one: all of it if it's behind a reference, as that's
    /// always all there; what's there of it, if the body owns it.
    fn drop_at(&mut self, out: &mut Out, st: &State, place: &Place, behind: bool, span: Span) {
        match behind {
            true => self.drop_whole(out, place, span),
            false => {
                let m = self.path(place);
                self.drop_path(out, st, m, span)
            }
        }
    }

    /// Sets the flags of `m` and every move path inside it, after
    /// what's at `m` got a value (`init`) or lost it.
    fn flag_set(&mut self, out: &mut Out, m: usize, init: bool, span: Span) {
        for &d in &self.paths.subtree[m] {
            if let Some(&f) = self.flags.get(&d) {
                let kind = StmtKind::Assign {
                    place: Place::local(f),
                    value: Rvalue::Use(Operand::Const(Const::Bool(init))),
                    declare: false,
                };
                out.push(kind, span);
            }
        }
    }

    fn flag_moves(&mut self, out: &mut Out, value: &Rvalue, span: Span) {
        for o in value.operands() {
            if let Operand::Move(p) = o {
                let m = self.path(p);
                self.flag_set(out, m, false, span);
            }
        }
    }

    /// The flag of move path `m`, which may or may not hold its value.
    fn flag(&mut self, m: usize) -> Operand {
        self.wanted.insert(m);
        // The first time round, before flags are made, any local will do.
        let f = self.flags.get(&m).copied().unwrap_or(RETURN);
        Operand::Copy(Place::local(f))
    }

    /// May some of what's at move path `m` be there?
    fn maybe_there(&self, st: &State, m: usize) -> bool {
        self.paths.subtree[m].iter().any(|&d| st.init[d])
    }

    /// Writes `then`, run only if `cond`.
    fn guarded(&mut self, out: &mut Out, cond: Operand, then: impl FnOnce(&mut Self, &mut Out)) {
        let (yes, join) = (out.new_block(), out.new_block());
        out.terminate(Term::Branch(cond, Bb(yes), Bb(join)));
        out.cur = yes;
        then(self, out);
        out.terminate(Term::Goto(Bb(join)));
        out.cur = join;
    }

    /// Drops what's there of the value at move path `m`.
    fn drop_path(&mut self, out: &mut Out, st: &State, m: usize, span: Span) {
        let place = self.paths.places[m].clone();
        if !self.maybe_there(st, m) || self.is_copy(&self.place_ty(&place)) {
            return;
        }
        let inside = &self.paths.subtree[m][1..];
        let partly = inside.iter().any(|&d| st.uninit[d]);
        let own_self = self.own_discard && place == Place::local(Local(1));
        if !partly && !own_self {
            match st.uninit[m] {
                false => self.drop_whole(out, &place, span),
                true => {
                    let f = self.flag(m);
                    self.guarded(out, f, |e, out| e.drop_whole(out, &place, span));
                }
            }
            return;
        }
        let open = |e: &mut Self, out: &mut Out| match own_self {
            true => e.open(out, st, m, None, span),
            false => e.open_or_whole(out, st, m, span),
        };
        match st.uninit[m] {
            false => open(self, out),
            true if !st.init[m] => {}
            true => {
                let f = self.flag(m);
                self.guarded(out, f, open);
            }
        }
    }

    /// Drops the value at move path `m`, which is there but for what may be
    /// moved out of the move paths inside it: whole, by its own `discard`,
    /// if it has one and all of it is there, or else what's there of it.
    fn open_or_whole(&mut self, out: &mut Out, st: &State, m: usize, span: Span) {
        let place = self.paths.places[m].clone();
        let own = match self.place_ty(&place) {
            Type::Adt(symbol, ..) => self.b.cx.traits().own_discard(&symbol),
            _ => false,
        };
        let inside: Vec<usize> = self.paths.subtree[m][1..].to_vec();
        // Definitely taken apart: nothing calls its `discard`.
        if !own || inside.iter().any(|&d| !st.init[d]) {
            return self.open(out, st, m, None, span);
        }
        let maybe: Vec<usize> = inside.into_iter().filter(|&d| st.uninit[d]).collect();
        let (open, join) = (out.new_block(), out.new_block());
        for d in maybe {
            let next = out.new_block();
            let f = self.flag(d);
            out.terminate(Term::Branch(f, Bb(next), Bb(open)));
            out.cur = next;
        }
        self.drop_whole(out, &place, span);
        out.terminate(Term::Goto(Bb(join)));
        out.cur = open;
        self.open(out, st, m, None, span);
        out.terminate(Term::Goto(Bb(join)));
        out.cur = join;
    }

    /// Drops the parts of the value at move path `m` that are there, one by
    /// one, in order, as it's been taken apart: for an enum, those of
    /// `variant` if it's known, or else of the variant it is.
    fn open(&mut self, out: &mut Out, st: &State, m: usize, variant: Option<usize>, span: Span) {
        let place = self.paths.places[m].clone();
        let t = self.place_ty(&place);
        let part = |e: &mut Self, out: &mut Out, p: Place| match e.paths.index.get(&p) {
            Some(&d) => e.drop_path(out, st, d, span),
            None => e.drop_whole(out, &p, span),
        };
        match &t {
            Type::Box(_) => {
                part(self, out, place.project(Proj::Deref));
                out.push(StmtKind::Free(place), span);
            }
            Type::Array(_, n) => {
                let Type::Nat(n) = **n else {
                    unreachable!("an array of unknown length taken apart")
                };
                for i in 0..n as usize {
                    part(self, out, place.project(Proj::Field(i)));
                }
            }
            Type::Adt(symbol, ..) => match &self.b.adts()[symbol].kind {
                AdtKind::Struct(fields) => {
                    for i in 0..fields.len() {
                        part(self, out, place.project(Proj::Field(i)));
                    }
                }
                AdtKind::Enum(variants) => {
                    let sizes: Vec<usize> = variants.iter().map(|(_, fs)| fs.len()).collect();
                    let fields = |e: &mut Self, out: &mut Out, v: usize| {
                        for i in 0..sizes[v] {
                            part(e, out, place.project(Proj::Variant(v, i)));
                        }
                    };
                    if let Some(v) = variant {
                        return fields(self, out, v);
                    }
                    let join = out.new_block();
                    let mut pats = Vec::new();
                    let mut targets = Vec::new();
                    let start = out.cur;
                    for (v, n) in sizes.iter().enumerate() {
                        let bb = out.new_block();
                        out.cur = bb;
                        fields(self, out, v);
                        out.terminate(Term::Goto(Bb(join)));
                        pats.push(Pat::Variant(v, vec![Pat::Any; *n]));
                        targets.push(Bb(bb));
                    }
                    out.cur = start;
                    out.terminate(Term::Match(place.clone(), pats, targets));
                    out.cur = join;
                }
            },
            t => unreachable!("`{t}` taken apart"),
        }
    }

    /// Drops the value at `place`, all of which is there.
    fn drop_whole(&mut self, out: &mut Out, place: &Place, span: Span) {
        let t = self.place_ty(place);
        if !self.is_copy(&t) {
            out.push(StmtKind::Drop(place.clone(), t), span);
        }
    }

    /// Gives each flag its first value, before anything else: `true` for
    /// what's in a parameter, `false` for the rest. In a block of its own,
    /// if the first is a loop's head.
    fn initialise_flags(&self, body: &mut Body) {
        if self.flags.is_empty() {
            return;
        }
        let entry = self.entry(body);
        let stmts: Vec<Statement> = (self.flags.iter())
            .map(|(&m, &f)| Statement {
                kind: StmtKind::Assign {
                    place: Place::local(f),
                    value: Rvalue::Use(Operand::Const(Const::Bool(entry.init[m]))),
                    declare: false,
                },
                span: Span::new(0, 0),
            })
            .collect();
        let gets_back = (0..body.blocks.len()).any(|b| body.successors(Bb(b)).contains(&Bb(0)));
        if gets_back {
            let moved = body.blocks.len();
            let first = std::mem::take(&mut body.blocks[0]);
            body.blocks.push(first);
            for block in &mut body.blocks {
                match &mut block.term {
                    Some(Term::Goto(t)) => retarget(t, moved),
                    Some(Term::Branch(_, t, e)) => {
                        retarget(t, moved);
                        retarget(e, moved);
                    }
                    Some(Term::Match(_, _, ts)) => ts.iter_mut().for_each(|t| retarget(t, moved)),
                    _ => {}
                }
            }
            for h in &mut body.heads {
                retarget(h, moved);
            }
            body.blocks[0] = Block {
                stmts,
                term: Some(Term::Goto(Bb(moved))),
            };
        } else {
            body.blocks[0].stmts.splice(0..0, stmts);
        }
    }
}

fn retarget(b: &mut Bb, to: usize) {
    if b.0 == 0 {
        *b = Bb(to);
    }
}
