//! MIR, the *mid-level intermediate representation* the borrow checker
//! works on, named after rustc's. A function body becomes a graph of basic
//! blocks: straight runs of simple statements, each ending in a jump. Every
//! value is in a numbered local, every step reads and writes *places* (a
//! local, maybe with fields and `*`s after it), and the order things happen
//! in, including which branch runs and where each variable and temporary
//! goes away, is spelled out. That's what makes questions like "is `x`
//! still borrowed here?" answerable: here is a point in the graph.
//!
//! It's also what runs. The borrow checker builds it from each function's
//! checked body (`borrowck/build.rs`), at the function's own generic types;
//! monomorphisation makes a copy of it for each list of type arguments
//! (`mono.rs`), and the interpreter runs those (`interp.rs`). So what the
//! borrow checker checks is the program that runs, not another lowering of
//! the same source that had better agree with it. What only running needs,
//! like the values of constants and the node of each operator whose panic
//! the checker ruled out, rides along where the borrow checker ignores it.

use crate::ast::{BinOp, Expr, NodeId, UnOp};
use crate::error::Span;
use crate::traits::FnKind;
use crate::types::Type;
use std::fmt;

/// Whose body a MIR body is: a function's, by symbol, or a closure's, by
/// its lambda's node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    Fn(String),
    Closure(NodeId),
}

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

#[derive(Clone)]
pub(crate) struct Body {
    pub(crate) locals: Vec<LocalDecl>,
    pub(crate) blocks: Vec<Block>,
    /// How many parameters there are: locals `1..=params`.
    pub(crate) params: usize,
    /// For a closure's body, the variables it captured, as its first
    /// parameter's fields, in order.
    pub(crate) env: Vec<EnvVar>,
    /// Where closures made here borrow what they capture mutably, as they
    /// change it, by the span of the borrow: the change.
    pub(crate) changed_by_closures: Vec<Span>,
    /// Where closures made here borrow a reference uniquely, to change what
    /// it points to: by `&mut`, but needing it unique only, not `mut`.
    pub(crate) unique_by_closures: Vec<Span>,
    /// The first block of each loop, which the end of its body and each
    /// `continue` go back to. It comes before every block of the body.
    pub(crate) heads: Vec<Bb>,
    /// The points of the `Dead`s and `Assign`s that drop a value which may
    /// still be there, and whose type may reach a `discard` of its own:
    /// there, the drop uses the value (see `moves::drop_uses`).
    pub(crate) drop_uses: std::collections::HashSet<Point>,
    /// The refinements its `let`s give their variables, which
    /// `StmtKind::Refine` checks as it runs.
    pub(crate) refinements: Vec<Refinement>,
    /// Where its `unsafe` blocks are: an unsafe operation called from
    /// inside one is in it.
    pub(crate) unsafe_blocks: Vec<Span>,
}

/// A refinement a `let` gives its variable, `{ binder: T | pred }`, and
/// for each other variable named in `pred`, where it is there.
#[derive(Clone)]
pub(crate) struct Refinement {
    pub(crate) binder: String,
    pub(crate) pred: Expr,
    pub(crate) env: Vec<(String, Place)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalKind {
    Return,
    Param,
    /// A variable the program declares.
    Var,
    /// The closure whose body this is, its first parameter: by `&`, by
    /// `&mut` or by value, as it's `Fn`, `FnMut` or `FnOnce`. What it
    /// captured are its fields (see `Body::env`).
    Env,
    /// A value the program doesn't name, like a function's result before
    /// it's used.
    Temp,
}

/// A variable a closure captured: how, and as what it's declared where the
/// closure is, which says if it may be changed.
#[derive(Debug, Clone)]
pub(crate) struct EnvVar {
    pub(crate) name: String,
    pub(crate) capture: crate::ast::Capture,
    /// Its id (see `check/typed.rs`).
    pub(crate) id: crate::ast::NodeId,
    /// Where it's declared.
    pub(crate) span: Span,
    pub(crate) mutable: bool,
}

#[derive(Clone)]
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
    /// For a variable bound by reference (match ergonomics), whether it's
    /// `&mut`.
    pub(crate) by_ref: Option<bool>,
    /// Declared with no value, `let x;`: an assignment gives it its first
    /// one, whether it's `mut` or not.
    pub(crate) deferred: bool,
}

#[derive(Default, Clone)]
pub(crate) struct Block {
    pub(crate) stmts: Vec<Statement>,
    pub(crate) term: Option<Term>,
}

#[derive(Clone)]
pub(crate) struct Statement {
    pub(crate) kind: StmtKind,
    pub(crate) span: Span,
}

#[derive(Clone)]
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
    /// The local has just been given a value that must have refinement
    /// `.1` of `Body::refinements`, and keep it whenever it's assigned to.
    /// Only the interpreter looks: it's what the refinement checker proved.
    Refine(Local, usize),
    /// A pattern has taken apart the value at the place, by value, and
    /// left it there, not used up, as the borrow checker knew its type to
    /// be `Discard`. Only the interpreter looks: it checks that it is, at
    /// the type it turned out to be.
    Kept(Place),
}

#[derive(Clone)]
pub(crate) enum Term {
    Goto(Bb),
    /// To the first block if the operand is `true`, else the second.
    Branch(Operand, Bb, Bb),
    /// To the block of the first of the patterns the value at the place
    /// matches: a `match`'s arms, or which alternative of an or-pattern
    /// binds its variables. The checker made sure one does. What the
    /// patterns test, the `Inspect`s before read, for the borrow checker.
    Match(Place, Vec<Pat>, Vec<Bb>),
    Return,
    /// Control never gets here: after a call of a function that returns
    /// `!`.
    Unreachable,
}

/// What a pattern tests of a value, by index rather than by name: all
/// running needs. Integer patterns match characters too, by their scalar
/// values.
#[derive(Debug, Clone)]
pub(crate) enum Pat {
    /// Anything, as `_` and a variable do.
    Any,
    /// A number in `lo..=hi`, either end maybe open.
    Range(Option<i128>, Option<i128>),
    Bool(bool),
    /// Variant `.0` of an enum, with patterns for its fields.
    Variant(usize, Vec<Pat>),
    /// Patterns for some of a struct's fields, by index.
    Struct(Vec<(usize, Pat)>),
    /// Patterns for each of an array's values.
    Array(Vec<Pat>),
    Or(Vec<Pat>),
    /// What the reference there points to matches (match ergonomics).
    Deref(Box<Pat>),
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

#[derive(Clone)]
pub(crate) enum Rvalue {
    Use(Operand),
    /// `&'r place` or `&'r mut place`, with region variable `'r`.
    Ref(usize, bool, Place),
    /// With the operator's node: whether the checker proved it can't
    /// panic is `Info::proved`'s, by node.
    Unary(UnOp, Operand, NodeId),
    /// `==` and `!=` read their operands without using them up: those are
    /// `Copy`s, whatever their types. So do `<` and the like, which on
    /// what isn't an integer call `Ord`'s method, by reference.
    Binary(BinOp, Operand, Operand, NodeId),
    /// `x as T`, to type `T`: the same number, or the character it's the
    /// scalar value of.
    Cast(Operand, Type, NodeId),
    /// A struct, variant, box, array or closure, of the type, from its
    /// parts.
    Aggregate(Agg, Type, Vec<Operand>),
    Call(Operand, Vec<Operand>),
}

/// What an `Rvalue::Aggregate` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Agg {
    /// A struct, its fields in declaration order.
    Struct,
    /// Variant `.0` of an enum.
    Variant(usize),
    /// `Box::new(x)`.
    Box,
    /// `[a, b, c]`.
    Array,
    /// `[v; n]`, of the length the type says.
    Repeat,
    /// The closure made by the lambda with this node, of what it captures.
    Closure(NodeId),
}

impl Rvalue {
    pub(crate) fn operands(&self) -> Vec<&Operand> {
        match self {
            Rvalue::Use(o) | Rvalue::Unary(_, o, _) | Rvalue::Cast(o, ..) => vec![o],
            Rvalue::Ref(..) => Vec::new(),
            Rvalue::Binary(_, a, b, _) => vec![a, b],
            Rvalue::Aggregate(_, _, os) => os.iter().collect(),
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
    Const(Const),
}

/// A value that doesn't come from a place.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Const {
    /// An integer, of its type: an integer type, or in generic code,
    /// maybe one that's a type parameter's.
    Int(i128, Type),
    Bool(bool),
    Char(char),
    Unit,
    /// A string literal, a `&str`.
    Str(String),
    /// A byte string literal.
    Bytes(Vec<u8>),
    /// A function, by symbol, at type arguments. A method of a trait, at
    /// `Self` first, is the impl's, once monomorphisation knows `Self`.
    Fn(String, Vec<Type>),
    /// The value of a const parameter of the function: a `usize`.
    Param(String),
    /// What's called is the call's first argument, its receiver: a
    /// closure, or what a bound says can be called, by `&` for `Fn`, `&mut`
    /// for `FnMut`, or by value for `FnOnce`.
    Receiver(FnKind),
}

impl Operand {
    pub(crate) fn place(&self) -> Option<&Place> {
        match self {
            Operand::Copy(p) | Operand::Move(p) => Some(p),
            Operand::Const(_) => None,
        }
    }
}

impl Body {
    /// The body as it runs: with the types' regions erased, which matter
    /// only to the borrow checker.
    pub(crate) fn erased(&self) -> Body {
        Body {
            locals: (self.locals.iter())
                .map(|l| LocalDecl {
                    ty: l.ty.erased(),
                    name: l.name.clone(),
                    mut_span: l.mut_span,
                    ..*l
                })
                .collect(),
            blocks: self.blocks.clone(),
            params: self.params,
            env: self.env.clone(),
            changed_by_closures: Vec::new(),
            unique_by_closures: Vec::new(),
            heads: self.heads.clone(),
            drop_uses: Default::default(),
            refinements: self.refinements.clone(),
            unsafe_blocks: self.unsafe_blocks.clone(),
        }
    }

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
            Some(Term::Match(_, _, ts)) => ts.clone(),
            Some(Term::Return | Term::Unreachable) | None => Vec::new(),
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
                    StmtKind::Refine(l, r) => writeln!(f, "    refine _{} {r}", l.0)?,
                    StmtKind::Kept(p) => writeln!(f, "    kept {p:?}")?,
                }
            }
            match &block.term {
                Some(Term::Goto(t)) => writeln!(f, "    goto bb{}", t.0)?,
                Some(Term::Branch(o, t, e)) => {
                    writeln!(f, "    if {o:?} bb{} else bb{}", t.0, e.0)?
                }
                Some(Term::Match(p, pats, ts)) => writeln!(f, "    match {p:?} {pats:?} {ts:?}")?,
                Some(Term::Return) => writeln!(f, "    return")?,
                Some(Term::Unreachable) => writeln!(f, "    unreachable")?,
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
        Rvalue::Unary(op, o, _) => format!("{op}{o:?}"),
        Rvalue::Binary(op, a, b, _) => format!("{a:?} {op} {b:?}"),
        Rvalue::Cast(o, t, _) => format!("{o:?} as {t}"),
        Rvalue::Aggregate(k, t, os) => format!("{k:?}<{t}>{os:?}"),
        Rvalue::Call(c, args) => format!("call {c:?}{args:?}"),
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// How `t mono` names what a body's aggregates make: struct fields and
/// enum variants, by their types' copies' names and their indices.
pub(crate) trait Names {
    fn field(&self, ty: &str, i: usize) -> Option<String>;
    fn variant(&self, ty: &str, i: usize) -> Option<String>;
}

impl Body {
    /// The body as `t mono` prints it, each line indented by `indent`: its
    /// locals, then its blocks, those control can't get to left out.
    pub(crate) fn render(&self, indent: &str, names: &dyn Names) -> String {
        let mut out = String::new();
        for (i, l) in self.locals.iter().enumerate() {
            let m = if l.mutable && l.kind != LocalKind::Temp && i > 0 {
                "mut "
            } else {
                ""
            };
            let what = match (&l.name, l.kind) {
                (Some(n), _) => format!(" // {n}"),
                (None, LocalKind::Env) => " // the closure".to_string(),
                _ => String::new(),
            };
            out.push_str(&format!("{indent}let {m}_{i}: {};{what}\n", l.ty));
        }
        for (b, block) in self.blocks.iter().enumerate() {
            let Some(term) = &block.term else { continue };
            out.push_str(&format!("{indent}bb{b}: {{\n"));
            let inner = format!("{indent}    ");
            for s in &block.stmts {
                let line = match &s.kind {
                    StmtKind::Assign { place, value, .. } => {
                        format!("{} = {}", show_place(place), show_rv(value, names))
                    }
                    StmtKind::Inspect(p) => format!("inspect {}", show_place(p)),
                    StmtKind::Dead(l) => format!("dead _{}", l.0),
                    StmtKind::Consume(p, _) => format!("consume {}", show_place(p)),
                    StmtKind::Kept(p) => format!("kept {}", show_place(p)),
                    StmtKind::Refine(l, r) => {
                        let r = &self.refinements[*r];
                        let pred = crate::pretty::expr_text(&r.pred);
                        format!("refine _{} {{ {} | {pred} }}", l.0, r.binder)
                    }
                };
                out.push_str(&format!("{inner}{line};\n"));
            }
            let line = match term {
                Term::Goto(t) => format!("goto bb{}", t.0),
                Term::Branch(o, t, e) => {
                    format!("if {} {{ bb{} }} else {{ bb{} }}", show_op(o), t.0, e.0)
                }
                Term::Match(p, pats, ts) => {
                    let arms: Vec<String> = (pats.iter().zip(ts))
                        .map(|(pat, t)| format!("{} => bb{}", show_pat(pat), t.0))
                        .collect();
                    format!("match {} {{ {} }}", show_place(p), arms.join(", "))
                }
                Term::Return => "return".to_string(),
                Term::Unreachable => "unreachable".to_string(),
            };
            out.push_str(&format!("{inner}{line};\n{indent}}}\n"));
        }
        out
    }
}

fn show_place(p: &Place) -> String {
    let mut s = format!("_{}", p.local.0);
    for proj in &p.proj {
        s = match proj {
            Proj::Deref => format!("(*{s})"),
            Proj::Field(i) => format!("{s}.{i}"),
            Proj::Variant(v, i) => format!("({s} as #{v}).{i}"),
        };
    }
    s
}

fn show_op(o: &Operand) -> String {
    match o {
        Operand::Copy(p) => format!("copy {}", show_place(p)),
        Operand::Move(p) => format!("move {}", show_place(p)),
        Operand::Const(c) => match c {
            Const::Int(n, t) => format!("{n}_{t}"),
            Const::Bool(b) => b.to_string(),
            Const::Char(c) => format!("{c:?}"),
            Const::Unit => "()".to_string(),
            Const::Str(s) => format!("{s:?}"),
            Const::Bytes(b) => format!("b{:?}", String::from_utf8_lossy(b)),
            Const::Fn(name, args) if args.is_empty() => name.clone(),
            Const::Fn(name, args) => {
                let args: Vec<String> = args.iter().map(Type::to_string).collect();
                format!("{name}::<{}>", args.join(", "))
            }
            Const::Param(name) => name.clone(),
            Const::Receiver(_) => "<receiver>".to_string(),
        },
    }
}

fn show_ops(os: &[Operand]) -> String {
    os.iter().map(show_op).collect::<Vec<_>>().join(", ")
}

fn show_rv(r: &Rvalue, names: &dyn Names) -> String {
    match r {
        Rvalue::Use(o) => show_op(o),
        Rvalue::Ref(_, m, p) => format!("&{}{}", if *m { "mut " } else { "" }, show_place(p)),
        Rvalue::Unary(op, o, _) => format!("{op}{}", show_op(o)),
        Rvalue::Binary(op, a, b, _) => format!("{} {op} {}", show_op(a), show_op(b)),
        Rvalue::Cast(o, t, _) => format!("{} as {t}", show_op(o)),
        Rvalue::Aggregate(k, t, os) => {
            let copy = match t {
                Type::Adt(name, _, args) => crate::mono::mangle(name, args),
                t => t.to_string(),
            };
            match k {
                Agg::Struct => {
                    let fields: Vec<String> = (os.iter().enumerate())
                        .map(|(i, o)| {
                            let f = names.field(&copy, i).unwrap_or_else(|| i.to_string());
                            format!("{f}: {}", show_op(o))
                        })
                        .collect();
                    format!("{copy} {{ {} }}", fields.join(", "))
                }
                Agg::Variant(i) => {
                    let v = names.variant(&copy, *i).unwrap_or_else(|| format!("#{i}"));
                    match os.is_empty() {
                        true => format!("{copy}::{v}"),
                        false => format!("{copy}::{v}({})", show_ops(os)),
                    }
                }
                Agg::Box => format!("Box::new({})", show_ops(os)),
                Agg::Array => format!("[{}]", show_ops(os)),
                Agg::Repeat => match t {
                    Type::Array(_, n) => format!("[{}; {n}]", show_ops(os)),
                    _ => format!("[{}; ?]", show_ops(os)),
                },
                Agg::Closure(id) => format!("{{closure#{}}}({})", id.0, show_ops(os)),
            }
        }
        Rvalue::Call(f, args) => match f {
            Operand::Const(Const::Receiver(_)) => match args.split_first() {
                Some((recv, rest)) => format!("({})({})", show_op(recv), show_ops(rest)),
                None => "<receiver>()".to_string(),
            },
            f => format!("{}({})", show_op(f), show_ops(args)),
        },
    }
}

fn show_pat(p: &Pat) -> String {
    match p {
        Pat::Any => "_".to_string(),
        Pat::Range(Some(a), Some(b)) if a == b => a.to_string(),
        Pat::Range(a, b) => format!(
            "{}..={}",
            a.map_or(String::new(), |a| a.to_string()),
            b.map_or(String::new(), |b| b.to_string())
        ),
        Pat::Bool(b) => b.to_string(),
        Pat::Variant(i, pats) if pats.is_empty() => format!("#{i}"),
        Pat::Variant(i, pats) => format!("#{i}({})", show_pats(pats)),
        Pat::Struct(fields) => {
            let fields: Vec<String> = (fields.iter())
                .map(|(i, p)| format!("{i}: {}", show_pat(p)))
                .collect();
            format!("{{ {} }}", fields.join(", "))
        }
        Pat::Array(pats) => format!("[{}]", show_pats(pats)),
        Pat::Or(alts) => alts.iter().map(show_pat).collect::<Vec<_>>().join(" | "),
        Pat::Deref(p) => format!("&{}", show_pat(p)),
    }
}

fn show_pats(ps: &[Pat]) -> String {
    ps.iter().map(show_pat).collect::<Vec<_>>().join(", ")
}
