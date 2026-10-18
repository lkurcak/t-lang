//! MIR, the *mid-level intermediate representation* the borrow checker
//! works on, named after rustc's. A function body becomes a graph of basic
//! blocks: straight runs of simple statements, each ending in a jump. Every
//! value is in a numbered local, every step reads and writes *places* (a
//! local, maybe with fields and `*`s after it), and the order things happen
//! in, including which branch runs and where each variable and temporary
//! goes away, is spelled out. That's what makes questions like "is `x`
//! still borrowed here?" answerable: here is a point in the graph.

use crate::ast::{BinOp, UnOp};
use crate::error::Span;
use crate::types::Type;
use std::fmt;

/// A local, by its index in `Body::locals`. `RETURN` holds what the body
/// returns; the parameters come next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Local(pub(crate) usize);

pub(crate) const RETURN: Local = Local(0);

/// A basic block, by its index in `Body::blocks`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Bb(pub(crate) usize);

/// A point in the body: before statement `index` of `block`, or before its
/// terminator, if `index` is how many statements it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Point {
    pub(crate) block: Bb,
    pub(crate) index: usize,
}

pub(crate) struct Body {
    pub(crate) locals: Vec<LocalDecl>,
    pub(crate) blocks: Vec<Block>,
    /// How many parameters there are: locals `1..=params`.
    pub(crate) params: usize,
    /// The first block of each loop, which the end of its body and each
    /// `continue` go back to. It comes before every block of the body.
    pub(crate) heads: Vec<Bb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalKind {
    Return,
    Param,
    /// A variable the program declares.
    Var,
    /// A variable a closure captured: its own copy, which it can't change.
    Captured,
    /// A value the program doesn't name, like a function's result before
    /// it's used.
    Temp,
}

pub(crate) struct LocalDecl {
    /// With its regions numbered (see `regions.rs`).
    pub(crate) ty: Type,
    pub(crate) kind: LocalKind,
    pub(crate) mutable: bool,
    /// The variable's name, if it has one.
    pub(crate) name: Option<String>,
    /// Where it's declared, or for a temporary, the expression it holds.
    pub(crate) span: Span,
    /// For a variable that's `mut`, where the `mut` is, from the `mut` to
    /// the name.
    pub(crate) mut_span: Option<Span>,
    /// Declared with no value, `let x;`: an assignment gives it its first
    /// one, whether it's `mut` or not.
    pub(crate) deferred: bool,
}

#[derive(Default)]
pub(crate) struct Block {
    pub(crate) stmts: Vec<Statement>,
    pub(crate) term: Option<Term>,
}

pub(crate) struct Statement {
    pub(crate) kind: StmtKind,
    pub(crate) span: Span,
}

pub(crate) enum StmtKind {
    /// Writes the value of the rvalue to the place. `declare` says it's a
    /// variable getting its first value, where it's declared, which it can
    /// have whether it's `mut` or not.
    Assign {
        place: Place,
        value: Rvalue,
        declare: bool,
    },
    /// Looks at what's in a place, without using it up: a `match` looking
    /// at which variant its scrutinee is.
    Inspect(Place),
    /// A local goes out of scope: what it holds is dropped, and its memory
    /// freed.
    Dead(Local),
    /// A value is *used up* by a pattern that takes it apart, where it is:
    /// a struct, or variant `.1` of an enum, that isn't `Discard`, matched
    /// by value. What the pattern's variables didn't move out of it is
    /// dropped, and then nothing's there. (See `moves::linear`.)
    Consume(Place, Option<usize>),
}

pub(crate) enum Term {
    Goto(Bb),
    /// To the first block if the operand is `true`, else the second.
    Branch(Operand, Bb, Bb),
    /// To one of the blocks, say a `match`'s arms: which one isn't worked
    /// out here, as the patterns' tests are `Inspect`s that came before.
    Switch(Vec<Bb>),
    Return,
}

/// Somewhere a value is: a local, and then fields and `*`s.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Place {
    pub(crate) local: Local,
    pub(crate) proj: Vec<Proj>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Proj {
    /// What a reference or box points to.
    Deref,
    /// Field `.0` of a struct, in declaration order.
    Field(usize),
    /// Field `.1` of variant `.0` of an enum.
    Variant(usize, usize),
}

impl Place {
    pub(crate) fn local(local: Local) -> Place {
        Place {
            local,
            proj: Vec::new(),
        }
    }

    pub(crate) fn project(&self, p: Proj) -> Place {
        let mut place = self.clone();
        place.proj.push(p);
        place
    }

    /// Is this `other`, or a part of it?
    pub(crate) fn within(&self, other: &Place) -> bool {
        self.local == other.local && self.proj.starts_with(&other.proj)
    }
}

pub(crate) enum Rvalue {
    Use(Operand),
    /// `&'r place` or `&'r mut place`, with region variable `'r`.
    Ref(usize, bool, Place),
    Unary(UnOp, Operand),
    /// `==` and `!=` read their operands without using them up: those are
    /// `Copy`s, whatever their types.
    Binary(BinOp, Operand, Operand),
    /// A struct, variant, box or closure, from its parts.
    Aggregate(Vec<Operand>),
    Call(Operand, Vec<Operand>),
}

impl Rvalue {
    pub(crate) fn operands(&self) -> Vec<&Operand> {
        match self {
            Rvalue::Use(o) | Rvalue::Unary(_, o) => vec![o],
            Rvalue::Ref(..) => Vec::new(),
            Rvalue::Binary(_, a, b) => vec![a, b],
            Rvalue::Aggregate(os) => os.iter().collect(),
            Rvalue::Call(f, args) => std::iter::once(f).chain(args).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Operand {
    /// Reads the value at the place, leaving it there.
    Copy(Place),
    /// Takes the value out of the place, which has nothing in it after.
    Move(Place),
    /// A value that doesn't come from a place: a literal, or a function.
    Const,
}

impl Operand {
    pub(crate) fn place(&self) -> Option<&Place> {
        match self {
            Operand::Copy(p) | Operand::Move(p) => Some(p),
            Operand::Const => None,
        }
    }
}

impl Body {
    /// Every point, in order: each block's statements and terminator.
    pub(crate) fn points(&self) -> impl Iterator<Item = Point> + '_ {
        self.blocks.iter().enumerate().flat_map(|(b, block)| {
            (0..=block.stmts.len()).map(move |index| Point {
                block: Bb(b),
                index,
            })
        })
    }

    /// The blocks control can go to after `b`.
    pub(crate) fn successors(&self, b: Bb) -> Vec<Bb> {
        match &self.blocks[b.0].term {
            Some(Term::Goto(t)) => vec![*t],
            Some(Term::Branch(_, t, f)) => vec![*t, *f],
            Some(Term::Switch(ts)) => ts.clone(),
            Some(Term::Return) | None => Vec::new(),
        }
    }

    /// Can control get from point `from` to point `to` without going back
    /// round a loop, to its head from inside it? Then what happens at
    /// `from` can happen before `to` in the same time round.
    pub(crate) fn same_round(&self, from: Point, to: Point) -> bool {
        if from.block == to.block && from.index < to.index {
            return true;
        }
        let back = |b: Bb, s: Bb| self.heads.contains(&s) && s.0 < b.0;
        let mut seen = vec![false; self.blocks.len()];
        let mut todo = vec![from.block];
        while let Some(b) = todo.pop() {
            for s in self.successors(b) {
                if back(b, s) || std::mem::replace(&mut seen[s.0], true) {
                    continue;
                }
                if s == to.block {
                    return true;
                }
                todo.push(s);
            }
        }
        false
    }

    /// The blocks that can come right before each block.
    pub(crate) fn predecessors(&self) -> Vec<Vec<Bb>> {
        let mut preds = vec![Vec::new(); self.blocks.len()];
        for b in 0..self.blocks.len() {
            for s in self.successors(Bb(b)) {
                preds[s.0].push(Bb(b));
            }
        }
        preds
    }
}

impl fmt::Display for Body {
    /// For debugging the borrow checker.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, l) in self.locals.iter().enumerate() {
            let name = l.name.as_deref().unwrap_or("");
            writeln!(f, "let _{i}: {:?} // {name} {:?}", l.ty, l.kind)?;
        }
        for (b, block) in self.blocks.iter().enumerate() {
            writeln!(f, "bb{b}:")?;
            for s in &block.stmts {
                match &s.kind {
                    StmtKind::Assign { place, value, .. } => {
                        writeln!(f, "    {place:?} = {}", show_rvalue(value))?
                    }
                    StmtKind::Inspect(p) => writeln!(f, "    inspect {p:?}")?,
                    StmtKind::Dead(l) => writeln!(f, "    dead _{}", l.0)?,
                    StmtKind::Consume(p, v) => writeln!(f, "    consume {p:?} {v:?}")?,
                }
            }
            match &block.term {
                Some(Term::Goto(t)) => writeln!(f, "    goto bb{}", t.0)?,
                Some(Term::Branch(o, t, e)) => {
                    writeln!(f, "    if {o:?} bb{} else bb{}", t.0, e.0)?
                }
                Some(Term::Switch(ts)) => writeln!(f, "    switch {ts:?}")?,
                Some(Term::Return) => writeln!(f, "    return")?,
                None => writeln!(f, "    <none>")?,
            }
        }
        Ok(())
    }
}

fn show_rvalue(r: &Rvalue) -> String {
    match r {
        Rvalue::Use(o) => format!("{o:?}"),
        Rvalue::Ref(v, m, p) => format!("&'{v} {}{p:?}", if *m { "mut " } else { "" }),
        Rvalue::Unary(op, o) => format!("{op}{o:?}"),
        Rvalue::Binary(op, a, b) => format!("{a:?} {op} {b:?}"),
        Rvalue::Aggregate(os) => format!("aggregate{os:?}"),
        Rvalue::Call(c, args) => format!("call {c:?}{args:?}"),
    }
}
