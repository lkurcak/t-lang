//! The interpreter. It gives the language its meaning (its *dynamic
//! semantics*) so we can check that the type system is right, and it's
//! what a backend has to agree with.
//!
//! It runs MIR (`mir.rs`): the borrow checker builds each function's from
//! its checked body and checks it, monomorphisation copies it for each
//! list of type arguments, and this runs the copies, a basic block at a
//! time. So what runs is what the borrow checker checked, the order things
//! happen in and where each value goes away included (docs/32-mir.md).
//!
//! Look at how often this file calls `unsound(..)`: every place where a value
//! could have the "wrong shape", like adding a bool, calling an int, an `if`
//! on a number, or a `match` where no arm fits. The type checker promises
//! those places are unreachable for every program it accepts. That promise
//! is the *type soundness* theorem ("well-typed programs don't go wrong"),
//! and those calls are where you'd see it fail if the checker had a bug.
//!
//! Since step 5 it also keeps memory the way a compiled program would, so
//! that the borrow checker's promises can fail here too. Every local lives
//! in a *slot*, made when it's first given a value and freed when it goes
//! out of scope (`StmtKind::Dead`); a box's value lives in a slot on the
//! *heap*, freed when the box is dropped. A reference is the address of a
//! slot and the path to a field in it, so following one to a slot that was
//! freed is a *dangling reference*. Moving a value out of a place leaves
//! `Value::Moved` behind, so using it again is a *use after move*, and
//! freeing a box twice a *double free*. A function that returns with a
//! local still in a slot is a bug in the lowering, and at the end, the heap
//! must be empty: anything left is a *leak*. The borrow checker promises
//! none of those happen (docs/05-ownership.md).
//!
//! Since step 6a.1 it keeps linear values the same way. What moves and
//! what's copied, and which patterns use up what they take apart, is what
//! the borrow checker decided, by the types it knew: in generic code, a
//! value of a type parameter that isn't known to be `Copy` moves, even
//! where it's an `i64`, so that a body means the same at every type. The
//! types monomorphisation made, which say which are `Copy` and which
//! `Discard`, only check those decisions: a copy of a value whose type isn't
//! `Copy`, or a value taken apart by value and not used up whose type isn't
//! `Discard` (`StmtKind::Kept`), is a checker bug. Throwing away whole a
//! value that isn't `Discard`, rather than using it up, is a leak too
//! (docs/06a.1-linear-types.md). Except on the way out of a run-time error:
//! that's a panic, which aborts, so nothing is really dropped.
//!
//! What's dropped where was worked out before it runs, by drop elaboration
//! (`borrowck/drops.rs`): a `drop` runs its type's glue, which
//! monomorphisation wrote (`mono/glue.rs`), sometimes behind a drop flag.
//! What this still sees, what's in each slot, checks that: a `drop` of what
//! isn't all there, or a `dead`, assignment or pattern leaving behind what
//! needed dropping, is a bug in elaboration (docs/32.1-drops.md).

use crate::ast::*;
use crate::error::{Error, Span};
use crate::mono::{glue_name, is_glue};
use crate::types::IntTy;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Deep enough for any reasonable recursion, shallow enough to stop runaway
/// recursion with an error instead of crashing the interpreter itself.
const MAX_CALL_DEPTH: usize = 10_000;

/// What a call says of where an abort under it is shown (see
/// `Interp::blamed`).
#[derive(Debug, Clone, Copy)]
enum Blame {
    /// Where it happens: what was called doesn't blame its caller.
    No,
    /// At the call, and further out if that's in a body that blames its
    /// caller too.
    Caller,
    /// At the call, and nowhere further out: a call through a value, whose
    /// site is where the value was made.
    Here,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer, of its type: always one of that type's values, here on
    /// a 64-bit machine.
    Int(i128, IntTy),
    Bool(bool),
    /// A character: a Unicode scalar value, as Rust's `char` is.
    Char(char),
    Unit,
    /// A function, by name. It can only see its own parameters and other
    /// functions. Nested ones have been lifted to the top level by now,
    /// under their symbols (see `lift.rs`). For one that blames its caller,
    /// or an intrinsic, where it was made a value: its caller, wherever
    /// it's called through the value (docs/34-blame-caller.md, B3).
    Fn(String, Option<Span>),
    /// A closure: an anonymous function, plus the local variables it can see.
    Closure(Closure),
    /// A struct: its type's name, and its fields in declaration order.
    Struct(String, Vec<(String, Value)>),
    /// An enum value: its type's name, which variant, and that variant's fields.
    Variant(String, String, Vec<Value>),
    /// A reference, `&` or `&mut`: where the value it points to is.
    Ref(Ptr),
    /// A box: where its value is, on the heap.
    Box(Ptr),
    /// A `RawBuf`: where its slots are, on the heap, as a `Slots`.
    RawBuf(Ptr),
    /// An `Arena`: where its values are, on the heap, as a `Slots` of a
    /// `Box` for each, in the order they were put in.
    Arena(Ptr),
    /// The slots of a `RawBuf`, each a value, or `Moved` if it's
    /// uninitialised, on the heap, where a `RawBuf` points. Or an array's
    /// values, `[T; N]`, wherever the array is, exactly `N` of them (lemma
    /// A1 of docs/22-arrays.md), each `Moved` once a pattern has moved it
    /// out.
    Slots(Vec<Value>),
    /// What's left in a place whose value was moved out.
    Moved,
}

/// Where a value is: a slot, and the fields to follow inside the value
/// there, each by its index among its struct's or variant's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Ptr {
    slot: usize,
    /// Which use of the slot it points into. Each time a slot is freed, it
    /// counts up, so a pointer from before isn't fooled by the slot's next
    /// use.
    generation: u64,
    path: Vec<usize>,
    /// For a slice, `[T]`: which of the `Slots` there it is, as the index
    /// of the first and how many. A reference to a slice is one with this.
    range: Option<(usize, usize)>,
}

impl Ptr {
    /// Field `i` of the value this points to.
    fn field(&self, i: usize) -> Ptr {
        let mut p = self.clone();
        p.path.push(i);
        p
    }

    /// Slots `lo..lo + len` of the `Slots` this points to, as a slice.
    fn slice(&self, lo: usize, len: usize) -> Ptr {
        Ptr {
            range: Some((lo, len)),
            ..self.clone()
        }
    }
}

/// What `|x| x + n` evaluates to: its code, the name of its body's copy
/// (see `mono::closure_copy`), and what it captured, its *environment*:
/// for each variable, a reference to it, or its value, moved or copied in,
/// as `Lambda::modes` says (see docs/06c-closures.md). Like a struct's
/// fields, as that's what it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub code: String,
    pub env: Vec<(String, Value)>,
}

/// How the body of a closure takes the closure: `Some(Some(m))` by
/// reference, `&mut` if `m`, as it's `Fn` or `FnMut`; `Some(None)` by
/// value, as it's `FnOnce`; `None` not at all, as it captures nothing.
fn env_by(body: &crate::mir::Body) -> Option<Option<bool>> {
    let env = body
        .locals
        .get(1)
        .filter(|l| l.kind == crate::mir::LocalKind::Env)?;
    Some(match &env.ty {
        crate::types::Type::Ref(_, m, _) => Some(*m),
        _ => None,
    })
}

/// The copy of a function that closure `code` is in.
fn closure_root(code: &str) -> &str {
    code.split("::{closure#").next().unwrap_or(code)
}

/// A function or closure being run: its body, the copy of the function
/// it's in, the function's name if it's a function's, and the slot of
/// each of its locals that has one.
struct Frame<'p> {
    body: &'p crate::mir::Body,
    root: &'p str,
    f: Option<&'p str>,
    slots: Vec<Option<Ptr>>,
}

/// Where a function or closure being run is: the `unsafe` blocks of its
/// body, the statement it's at, and the function's name, if it's one.
struct Site<'p> {
    unsafe_blocks: &'p [Span],
    span: Span,
    f: Option<&'p str>,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n, _) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Unit => f.write_str("()"),
            Value::Fn(name, _) => write!(f, "<fn {}>", crate::check::source_name(name)),
            Value::Closure(_) => f.write_str("<closure>"),
            Value::Ref(_) => f.write_str("<reference>"),
            Value::Box(_) => f.write_str("<box>"),
            Value::RawBuf(_) => f.write_str("<buffer>"),
            Value::Arena(_) => f.write_str("<arena>"),
            Value::Slots(_) => f.write_str("<slots>"),
            Value::Moved => f.write_str("<moved>"),
            Value::Struct(name, fields) if fields.is_empty() => {
                write!(f, "{} {{}}", crate::check::source_name(name))
            }
            Value::Struct(name, fields) => {
                write!(f, "{} {{ ", crate::check::source_name(name))?;
                for (i, (field, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{field}: {v}")?;
                }
                f.write_str(" }")
            }
            Value::Variant(ty, variant, fields) => {
                write!(f, "{}::{variant}", crate::check::source_name(ty))?;
                if !fields.is_empty() {
                    f.write_str("(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        write!(f, "{v}")?;
                    }
                    f.write_str(")")?;
                }
                Ok(())
            }
        }
    }
}

/// Runs `main`, with `term` as its streams. Anything the program prints goes there, even if the
/// program later fails.
pub fn run(program: &crate::mono::Executable, term: &mut dyn Terminal) -> Result<Value, Error> {
    run_entry(program, "main", term).result
}

/// How running an entry ended: its value or error, and whether it
/// aborted by a panic (not by running out of memory, or of stack).
pub struct Ended {
    pub result: Result<Value, Error>,
    pub panicked: bool,
}

/// Runs `entry`, `main` or a test, as the program: with its own memory,
/// and, for `main`, the `Os`. An error it gives back is reported, and
/// after it, everything must have been freed. Tests don't share anything:
/// each run starts from nothing (docs/31-tests.md, T2).
pub fn run_entry(program: &crate::mono::Executable, entry: &str, term: &mut dyn Terminal) -> Ended {
    let prog = program.program();
    let fns = prog.fns.iter().map(|f| (f.name.name.as_str(), f)).collect();
    let derived = |tr: &str| -> HashSet<&str> {
        let with = prog.types.iter().filter(|d| d.derive_of(tr).is_some());
        with.map(|d| d.name().name.as_str()).collect()
    };
    let structs = prog
        .types
        .iter()
        .filter_map(|t| match t {
            TypeDecl::Struct(s) => Some((s.name.name.as_str(), s)),
            TypeDecl::Enum(_) | TypeDecl::Alias(_) => None,
        })
        .collect();
    let enums = prog
        .types
        .iter()
        .filter_map(|t| match t {
            TypeDecl::Enum(e) => Some((e.name.name.as_str(), e)),
            TypeDecl::Struct(_) | TypeDecl::Alias(_) => None,
        })
        .collect();
    let mut memory = Memory::default();
    // Where every empty slice points: no slots, never freed.
    let nowhere = memory.alloc(Value::Slots(Vec::new()), false);
    let mut interp = Interp {
        fns,
        structs,
        enums,
        nowhere,
        literals: HashMap::new(),
        copy: derived("Copy"),
        discard: derived("Discard"),
        aborting: false,
        message: String::new(),
        panicked: false,
        calls: Vec::new(),
        memory,
        depth: 0,
        frames: Vec::new(),
        sites: Vec::new(),
        proved: &prog.proved,
        measures: &prog.measures,
        refined: HashMap::new(),
        exe: program,
        term,
    };
    let result = interp.entry(program, entry);
    Ended {
        result,
        panicked: interp.panicked,
    }
}

impl<'p> Interp<'p, '_> {
    /// `run_entry`'s work.
    fn entry(&mut self, program: &crate::mono::Executable, entry: &str) -> Result<Value, Error> {
        // `main` takes nothing, or the one `std::Os` there is.
        let args = match self.fns[entry].params.len() {
            0 => Vec::new(),
            _ => vec![Value::Struct(
                "std::Os".into(),
                ["stdin", "stdout", "stderr"]
                    .into_iter()
                    .zip(["std::io::Stdin", "std::io::Stdout", "std::io::Stderr"])
                    .map(|(field, ty)| (field.to_string(), Value::Struct(ty.into(), Vec::new())))
                    .collect(),
            )],
        };
        let result = self.call(Value::Fn(entry.to_string(), None), args, Span::new(0, 0))?;
        // An error it gives back is reported, by the copy of `std::rt::report`
        // monomorphisation made for it, with its own `Stderr`.
        if let Value::Variant(ty, variant, fields) = &result
            && ty.starts_with("core::result::Result::<")
            && variant == "Err"
        {
            let report = (program.report(entry))
                .unwrap_or_else(|| unsound("no `std::rt::report` for the entry's error"))
                .to_string();
            let [e] = fields.as_slice() else {
                unsound("an `Err` of one field")
            };
            let at = self.memory.alloc(e.clone(), false);
            let stderr = Value::Struct("std::io::Stderr".into(), Vec::new());
            self.call(
                Value::Fn(report, None),
                vec![Value::Ref(at.clone()), stderr],
                Span::new(0, 0),
            )?;
            self.memory.free(&at, false);
        }
        if let Some(glue) = program.entry_drop(entry) {
            self.call(
                Value::Fn(glue.to_string(), None),
                vec![result.clone()],
                Span::new(0, 0),
            )?;
        }
        if self.memory.on_heap > 0 {
            unsound(&format!(
                "memory leak: {} boxes or buffers were never freed",
                self.memory.on_heap
            ));
        }
        Ok(result)
    }
}

#[track_caller]
fn unsound(what: &str) -> ! {
    unreachable!("type soundness violated: {what}. This is a bug in the type checker.")
}

fn int(v: Value) -> i128 {
    match v {
        Value::Int(n, _) => n,
        other => unsound(&format!("expected an integer, got `{other}`")),
    }
}

/// `n`, as a `t`, which it must be one of.
fn int_of(n: i128, t: IntTy) -> Value {
    let (lo, hi) = t.range_64();
    if !(lo..=hi).contains(&n) {
        unsound(&format!("{n} as {}", t.a()));
    }
    Value::Int(n, t)
}

/// The `t` that `n` is congruent to modulo 2^w, if `t` is `w` bits wide:
/// `n` is right in its last `w` bits, which is all that's kept of a
/// product that overflowed `i128`, as `wrapping_mul` is.
fn wrap(n: i128, t: IntTy) -> i128 {
    let (lo, hi) = t.range_64();
    let modulus = hi - lo + 1;
    let m = n.rem_euclid(modulus);
    if m > hi { m - modulus } else { m }
}

/// A `usize`, from a count the interpreter keeps.
fn usize_of(n: usize) -> Value {
    int_of(n as i128, IntTy::Usize)
}

/// The integer type written `t`, after monomorphisation.
fn int_ty(t: &TypeExpr) -> IntTy {
    match &t.kind {
        TypeExprKind::Named(name, _) => {
            IntTy::named(name).unwrap_or_else(|| unsound(&format!("`{name}` as an integer type")))
        }
        _ => unsound("an integer type that isn't named"),
    }
}

/// Is function `f` a copy of an impl's `discard`?
fn is_discard_fn(f: &str) -> bool {
    f.ends_with(&format!(" as {}>::discard", crate::traits::DISCARD))
}

fn bool(v: Value) -> bool {
    match v {
        Value::Bool(b) => b,
        other => unsound(&format!("expected a bool, got `{other}`")),
    }
}

/// A place for a value to live: a variable or temporary on the stack, or
/// what a box holds, on the heap.
struct Slot {
    value: Value,
    generation: u64,
    live: bool,
}

/// Every slot, in use or not.
#[derive(Default)]
struct Memory {
    slots: Vec<Slot>,
    /// Slots not in use, to use again.
    free: Vec<usize>,
    /// How many slots are on the heap, for finding leaks.
    on_heap: usize,
}

impl Memory {
    /// A new slot holding `value`, on the heap if `heap`.
    fn alloc(&mut self, value: Value, heap: bool) -> Ptr {
        self.on_heap += usize::from(heap);
        let slot = match self.free.pop() {
            Some(i) => {
                let s = &mut self.slots[i];
                s.value = value;
                s.live = true;
                i
            }
            None => {
                self.slots.push(Slot {
                    value,
                    generation: 0,
                    live: true,
                });
                self.slots.len() - 1
            }
        };
        Ptr {
            slot,
            generation: self.slots[slot].generation,
            path: Vec::new(),
            range: None,
        }
    }

    /// Frees the slot `p` points to, which must be in use, and returns
    /// what was in it.
    fn free(&mut self, p: &Ptr, heap: bool) -> Value {
        self.slot(p);
        let s = &mut self.slots[p.slot];
        s.live = false;
        s.generation += 1;
        self.free.push(p.slot);
        self.on_heap -= usize::from(heap);
        std::mem::replace(&mut s.value, Value::Unit)
    }

    /// The slot `p` points into, which must still be there.
    fn slot(&self, p: &Ptr) -> &Slot {
        let s = &self.slots[p.slot];
        if !s.live || s.generation != p.generation {
            unsound("a reference to memory that was freed: a dangling reference, or a double free");
        }
        s
    }

    /// The value `p` points to.
    fn get(&self, p: &Ptr) -> &Value {
        let mut v = &self.slot(p).value;
        for &i in &p.path {
            v = match v {
                Value::Struct(_, fields) => &fields[i].1,
                Value::Variant(_, _, fields) => &fields[i],
                Value::Closure(c) => &c.env[i].1,
                Value::Slots(slots) => &slots[i],
                Value::Moved => unsound("use of a value that was moved"),
                other => unsound(&format!("field {i} of `{other}`")),
            };
        }
        v
    }

    fn get_mut(&mut self, p: &Ptr) -> &mut Value {
        self.slot(p);
        let mut v = &mut self.slots[p.slot].value;
        for &i in &p.path {
            v = match v {
                Value::Struct(_, fields) => &mut fields[i].1,
                Value::Variant(_, _, fields) => &mut fields[i],
                Value::Closure(c) => &mut c.env[i].1,
                Value::Slots(slots) => &mut slots[i],
                Value::Moved => unsound("assignment into a value that was moved"),
                other => unsound(&format!("field {i} of `{other}`")),
            };
        }
        v
    }
}

struct Interp<'p, 'o> {
    fns: HashMap<&'p str, &'p FnDecl>,
    /// For putting struct literals' fields in declaration order.
    structs: HashMap<&'p str, &'p StructDecl>,
    /// For the copies of `Option` that intrinsics make.
    enums: HashMap<&'p str, &'p EnumDecl>,
    /// The `Slots` every empty slice is of.
    nowhere: Ptr,
    /// Where each literal's bytes are, by what they are: slots of `u8`s,
    /// never freed nor changed, made the first time they're needed.
    literals: HashMap<Vec<u8>, Ptr>,
    /// The structs and enums that are `Copy`, and that are `Discard`, as
    /// monomorphisation's copies of them say.
    copy: HashSet<&'p str>,
    discard: HashSet<&'p str>,
    /// Is a run-time error on its way out? Then what's dropped isn't
    /// checked for leaks: a panic aborts.
    aborting: bool,
    /// The message `core::panic::panic_with` is writing.
    message: String,
    /// Whether the program aborted by a panic.
    panicked: bool,
    /// Where each call being run was called, outermost first, and whether
    /// what it called blames its caller: where a panic is shown (see
    /// `Interp::blamed`).
    calls: Vec<(Span, Blame)>,
    memory: Memory,
    depth: usize,
    /// The functions being run, outermost first, by name: whose effects
    /// each panic and allocation must be among (see `effect`).
    frames: Vec<String>,
    /// Where each function and closure being run is, innermost last (see
    /// `unsafe_op`).
    sites: Vec<Site<'p>>,
    /// The operators the checker proved can't panic, by node.
    proved: &'p HashSet<NodeId>,
    /// What the checker found of measures, for refinements that use them.
    measures: &'p crate::ast::Measures,
    /// The refinement of each refined variable, by its slot and the
    /// slot's generation: what each assignment to it must keep.
    refined: HashMap<(usize, u64), Refined>,
    /// What runs: the MIR of each copy of a function and closure.
    exe: &'p crate::mono::Executable,
    /// The operating system's streams, as the program's `std::Os` reaches
    /// them.
    term: &'o mut dyn Terminal,
}

/// The streams of the operating system, for the intrinsics of `std::io`'s
/// `Stdin`, `Stdout` and `Stderr` (see docs/19-std-io.md): the process's
/// own, for `t run`, or bytes in memory, for tests.
pub trait Terminal {
    /// The next byte of input, or `None` at its end.
    fn read_byte(&mut self) -> std::io::Result<Option<u8>>;

    /// Writes byte `b` to the output, or to where errors go if `err`.
    fn write_byte(&mut self, err: bool, b: u8) -> std::io::Result<()>;
}

/// Streams in memory: input given ahead, and what's written kept.
#[derive(Default)]
pub struct Captured {
    pub input: std::collections::VecDeque<u8>,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl Terminal for Captured {
    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        Ok(self.input.pop_front())
    }

    fn write_byte(&mut self, err: bool, b: u8) -> std::io::Result<()> {
        match err {
            true => self.err.push(b),
            false => self.out.push(b),
        }
        Ok(())
    }
}

/// What a stream's intrinsic gives back for `e`: 257 for a broken pipe,
/// 258 for anything else, as `std::io::error` reads it.
fn io_error(e: &std::io::Error) -> u32 {
    match e.kind() {
        std::io::ErrorKind::BrokenPipe => 257,
        _ => 258,
    }
}

/// A refinement to check at run time (see `check/refine.rs`): its binder,
/// predicate, and the values of the other names in it, which can't change.
#[derive(Clone)]
struct Refined {
    binder: String,
    pred: Expr,
    env: Vec<(String, Value)>,
    /// What each measure the predicate calls of a name other than its
    /// binder gives, by node: found when it's made, as the predicate is
    /// about then, and what's behind a reference may change after.
    measured: HashMap<NodeId, PredValue>,
}

/// What a predicate evaluates to: integers are the mathematical ones. A
/// value that's neither is only ever what a measure is of.
#[derive(Clone)]
enum PredValue {
    Int(i128),
    Bool(bool),
    Val(Value),
}

impl PredValue {
    fn int(self) -> i128 {
        match self {
            PredValue::Int(n) => n,
            _ => unsound("a refinement's `bool` used as an integer"),
        }
    }

    fn boolean(self) -> bool {
        match self {
            PredValue::Bool(b) => b,
            _ => unsound("a refinement's integer used as a `bool`"),
        }
    }

    fn of(v: &Value) -> PredValue {
        match v {
            Value::Int(n, _) => PredValue::Int(*n),
            Value::Bool(b) => PredValue::Bool(*b),
            v => PredValue::Val(v.clone()),
        }
    }

    fn val(self) -> Value {
        match self {
            PredValue::Val(v) => v,
            _ => unsound("a measure of an integer or a `bool`"),
        }
    }
}

/// `a op b` in a predicate, of integers in ℤ.
fn pred_op(op: BinOp, a: PredValue, b: PredValue) -> PredValue {
    match (op, a, b) {
        (BinOp::And, a, b) => PredValue::Bool(a.boolean() && b.boolean()),
        (BinOp::Or, a, b) => PredValue::Bool(a.boolean() || b.boolean()),
        (BinOp::Eq, PredValue::Bool(a), PredValue::Bool(b)) => PredValue::Bool(a == b),
        (BinOp::Ne, PredValue::Bool(a), PredValue::Bool(b)) => PredValue::Bool(a != b),
        (op, a, b) => {
            let (a, b) = (a.int(), b.int());
            let arith = |n: Option<i128>| match n {
                Some(n) => PredValue::Int(n),
                None => unsound("a refinement's arithmetic out of range"),
            };
            match op {
                BinOp::Add => arith(a.checked_add(b)),
                BinOp::Sub => arith(a.checked_sub(b)),
                BinOp::Mul => arith(a.checked_mul(b)),
                BinOp::Eq => PredValue::Bool(a == b),
                BinOp::Ne => PredValue::Bool(a != b),
                BinOp::Lt => PredValue::Bool(a < b),
                BinOp::Le => PredValue::Bool(a <= b),
                BinOp::Gt => PredValue::Bool(a > b),
                BinOp::Ge => PredValue::Bool(a >= b),
                _ => unsound(&format!("`{op}` in a refinement")),
            }
        }
    }
}

impl Refined {
    /// Stops the program, as the checker is wrong, unless it holds of `v`,
    /// which is `what`.
    fn check(&self, i: &Interp, v: &Value, what: &str) {
        if let PredValue::Bool(false) = self.eval(i, &self.pred, v) {
            unsound(&format!(
                "{what} is `{v}`, which doesn't have its refinement `{}`",
                crate::pretty::expr_text(&self.pred)
            ));
        }
    }

    fn eval(&self, i: &Interp, e: &Expr, v: &Value) -> PredValue {
        match &e.kind {
            ExprKind::Int(n, _) => PredValue::Int(i128::from(*n)),
            ExprKind::Bool(b) => PredValue::Bool(*b),
            ExprKind::Var(name, _) if *name == self.binder => PredValue::of(v),
            ExprKind::Var(name, _) => match self.env.iter().find(|(n, _)| n == name) {
                Some((_, v)) => PredValue::of(v),
                None => unsound(&format!("a refinement naming `{name}`, which isn't there")),
            },
            ExprKind::Unary(UnOp::Neg, x) => PredValue::Int(-self.eval(i, x, v).int()),
            ExprKind::Unary(UnOp::Not, x) => PredValue::Bool(!self.eval(i, x, v).boolean()),
            ExprKind::Binary(op, l, r) => pred_op(*op, self.eval(i, l, v), self.eval(i, r, v)),
            ExprKind::MethodCall(x, ..) => match self.measured.get(&e.id) {
                Some(m) => m.clone(),
                None => i.measure_call(e.id, self.eval(i, x, v).val()),
            },
            // `old(x)` in a postcondition: its measures were found at the
            // call.
            ExprKind::Call(_, args) if args.len() == 1 => self.eval(i, &args[0], v),
            _ => unsound("a refinement outside the logic"),
        }
    }
}

/// Each measure call in predicate `e`, by node, with its receiver: in a
/// postcondition (`post`), only those inside `old(..)`.
fn each_measure_call<'e>(e: &'e Expr, post: bool, out: &mut Vec<(NodeId, &'e Expr)>) {
    match &e.kind {
        ExprKind::MethodCall(x, ..) if !post => out.push((e.id, x)),
        ExprKind::Call(_, args) if args.len() == 1 => {
            return each_measure_call(&args[0], false, out);
        }
        _ => {}
    }
    for c in e.children() {
        each_measure_call(c, post, out);
    }
}

impl<'p> Interp<'p, '_> {
    /// The refinement `{ binder | pred }`, whose other names have the
    /// values `env`: what the measures it calls of them give is found now.
    fn refined(&self, binder: &str, pred: &Expr, env: Vec<(String, Value)>) -> Refined {
        self.refined_at(binder, pred, env, false)
    }

    /// The same, of a postcondition (`post`): what's behind its names is
    /// found when it's checked, at return, but for what's in `old(..)`.
    fn refined_at(
        &self,
        binder: &str,
        pred: &Expr,
        env: Vec<(String, Value)>,
        post: bool,
    ) -> Refined {
        let mut r = Refined {
            binder: binder.to_string(),
            pred: pred.clone(),
            env,
            measured: HashMap::new(),
        };
        let mut calls = Vec::new();
        each_measure_call(pred, post, &mut calls);
        for (node, x) in calls {
            if let ExprKind::Var(name, _) = &x.kind
                && *name != r.binder
            {
                let recv = r.eval(self, x, &Value::Unit).val();
                let m = self.measure_call(node, recv);
                r.measured.insert(node, m);
            }
        }
        r
    }

    /// What the measure call at `node`, in a predicate or a measure's body,
    /// gives of receiver `recv`, through the `*`s the checker found.
    fn measure_call(&self, node: NodeId, recv: Value) -> PredValue {
        let Some((symbol, derefs, by)) = self.measures.calls.get(&node) else {
            unsound("a measure the checker didn't find")
        };
        let peel = match by {
            Some(_) if *derefs > 0 => derefs - 1,
            _ => *derefs,
        };
        let this = (0..peel).fold(recv, |v, _| self.pred_deref(v));
        self.measure(symbol, this)
    }

    /// What a reference or box `v` points to, if it's one, but a slice's
    /// or a string's, whose measures are of it; or else `v`.
    fn pred_deref(&self, v: Value) -> Value {
        match &v {
            Value::Ref(p) | Value::Box(p) if p.range.is_none() => self.memory.get(p).clone(),
            _ => v,
        }
    }

    /// What measure `symbol` gives of `this`, its `&self`, or what that
    /// points to: its body's value, or an intrinsic's.
    fn measure(&self, symbol: &str, this: Value) -> PredValue {
        match self.measures.bodies.get(symbol) {
            Some(Some(body)) => {
                let term = match &body.kind {
                    ExprKind::Block(_, Some(tail)) => &**tail,
                    _ => body,
                };
                self.measure_term(term, &this)
            }
            Some(None) => match (symbol, &this) {
                (
                    "<[T]>::len" | "<str>::len",
                    Value::Ref(Ptr {
                        range: Some((_, len)),
                        ..
                    }),
                ) => PredValue::Int(*len as i128),
                ("<[T; N]>::len", this) => match self.pred_deref(this.clone()) {
                    Value::Slots(values) => PredValue::Int(values.len() as i128),
                    other => unsound(&format!("`len` of `{other}`, which isn't an array")),
                },
                ("<RawBuf<T>>::cap", this) => match self.pred_deref(this.clone()) {
                    Value::RawBuf(q) => match self.memory.get(&q) {
                        Value::Slots(slots) => PredValue::Int(slots.len() as i128),
                        other => unsound(&format!("a `RawBuf` of `{other}`")),
                    },
                    other => unsound(&format!("`cap` of `{other}`")),
                },
                (_, this) => unsound(&format!("intrinsic measure `{symbol}` of `{this}`")),
            },
            None => unsound(&format!("`{symbol}` isn't a measure")),
        }
    }

    /// The value of term `e` of a measure's body, whose `self` is `this`.
    fn measure_term(&self, e: &Expr, this: &Value) -> PredValue {
        match &e.kind {
            ExprKind::Int(n, _) => PredValue::Int(i128::from(*n)),
            ExprKind::Bool(b) => PredValue::Bool(*b),
            ExprKind::Var(..) => PredValue::Val(this.clone()),
            ExprKind::Field(x, name) => {
                let mut v = self.measure_term(x, this).val();
                while let Value::Ref(p) | Value::Box(p) = &v
                    && p.range.is_none()
                {
                    v = self.pred_deref(v);
                }
                match &v {
                    Value::Struct(_, fields) => {
                        match fields.iter().find(|(n, _)| *n == name.name) {
                            Some((_, f)) => PredValue::of(f),
                            None => unsound(&format!("no field `{}` in `{v}`", name.name)),
                        }
                    }
                    _ => unsound(&format!("field `{}` of `{v}`", name.name)),
                }
            }
            ExprKind::MethodCall(x, ..) => {
                let recv = self.measure_term(x, this).val();
                self.measure_call(e.id, recv)
            }
            ExprKind::Unary(UnOp::Neg, x) => PredValue::Int(-self.measure_term(x, this).int()),
            ExprKind::Unary(UnOp::Not, x) => PredValue::Bool(!self.measure_term(x, this).boolean()),
            ExprKind::Binary(op, l, r) => {
                pred_op(*op, self.measure_term(l, this), self.measure_term(r, this))
            }
            _ => unsound("a measure's body outside the logic"),
        }
    }

    /// The value at `p`, moved out if `moves`, or else copied, as the
    /// borrow checker decided. It must all be there. What it copies, it
    /// knew to be `Copy`, which a type parameter is only if every type it
    /// can be is; what it moves needn't be at the type it turned out to be.
    fn read(&mut self, p: &Ptr, moves: bool) -> Value {
        let v = self.memory.get(p);
        if self.taken_apart(v) {
            unsound("use of a value that was moved out of, at least in part");
        }
        if moves {
            std::mem::replace(self.memory.get_mut(p), Value::Moved)
        } else if !self.copyable(v) {
            unsound(&format!("a copy of `{v}`, whose type isn't `Copy`"));
        } else {
            v.clone()
        }
    }

    /// Is `v`'s type `Copy`? A box's never is, and a reference's always is,
    /// as far as memory goes: that a `&mut` isn't is the borrow checker's
    /// business.
    fn copyable(&self, v: &Value) -> bool {
        match v {
            Value::Box(_) | Value::RawBuf(_) | Value::Arena(_) => false,
            Value::Struct(ty, _) | Value::Variant(ty, ..) => self.copy.contains(ty.as_str()),
            // A closure is `Copy` if what it holds is.
            Value::Closure(c) => c.env.iter().all(|(_, v)| self.copyable(v)),
            _ => true,
        }
    }

    /// Has some of `v` been moved out of, maybe all of it? Through boxes
    /// too, which own what they point to.
    fn taken_apart(&self, v: &Value) -> bool {
        match v {
            Value::Moved => true,
            Value::Box(p) => self.taken_apart(self.memory.get(p)),
            Value::Struct(_, fields) => fields.iter().any(|(_, v)| self.taken_apart(v)),
            Value::Variant(_, _, fields) => fields.iter().any(|v| self.taken_apart(v)),
            Value::Closure(c) => c.env.iter().any(|(_, v)| self.taken_apart(v)),
            Value::Slots(values) => values.iter().any(|v| self.taken_apart(v)),
            _ => false,
        }
    }

    /// Drops `v`, of type `ty`, all of which must be there (lemma E1 of
    /// docs/32.1-drops.md): by its type's drop glue, which monomorphisation
    /// wrote, or for an `Arena`, by freeing its values, first put in first,
    /// each by the glue of its box.
    ///
    /// Throwing away a value that isn't `Discard` is a leak: the borrow
    /// checker said this drop never happens, or happens to what's been
    /// taken apart, which drop elaboration drops part by part instead.
    fn drop(&mut self, v: Value, ty: &crate::types::Type, span: Span) -> Result<(), Error> {
        if self.taken_apart(&v) {
            unsound(&format!(
                "a drop of `{v}`, which isn't all there: drop elaboration is wrong"
            ));
        }
        if let Value::RawBuf(_) = v {
            unsound("a `RawBuf` was thrown away, and it isn't `Discard`: a leak");
        }
        if let Value::Struct(ty, _) | Value::Variant(ty, ..) = &v
            && !self.discard.contains(ty.as_str())
        {
            unsound(&format!(
                "a `{}` was thrown away, and it isn't `Discard`: a leak",
                crate::check::source_name(ty)
            ));
        }
        if let Value::Arena(p) = v {
            let crate::types::Type::Adt(_, _, args) = ty else {
                unsound("an arena of no type")
            };
            let boxed = crate::types::Type::Box(Box::new(args[0].clone()));
            let Value::Slots(values) = self.memory.free(&p, true) else {
                unsound("an arena's values")
            };
            let mut result = Ok(());
            for v in values {
                let r = self.drop(v, &boxed, span);
                if result.is_ok() {
                    result = r;
                }
            }
            return result;
        }
        let name = glue_name("{drop}", ty);
        let exe = self.exe;
        match exe.body_named(&name) {
            Some((name, body)) => self
                .run(body, name, None, vec![v], Vec::new(), span)
                .map(|_| ()),
            None if self.needs_drop(&v) => unsound(&format!("no drop glue for `{v}`, as `{name}`")),
            None => Ok(()),
        }
    }

    /// The `discard` of a struct or enum's own, if type `ty` has one.
    ///
    /// Relies on monomorphisation having made a copy of it for every type
    /// that has one (`Mono::type_decl`).
    fn user_discard(&self, ty: &str) -> Option<String> {
        let name = format!("<{ty} as {}>::discard", crate::traits::DISCARD);
        self.fns.contains_key(name.as_str()).then_some(name)
    }

    /// The error at `span`, of a program that panics: from here on,
    /// nothing is dropped for real.
    fn panic(&mut self, span: Span, message: impl Into<String>) -> Error {
        self.aborting = true;
        Error::runtime(self.blamed(span), message)
    }

    /// Where an abort at `span` is shown: there, unless it's directly in
    /// the body of a function that blames its caller, which it then is
    /// where that was called, and so on out (docs/34-blame-caller.md, B2).
    /// A closure's body, and one of a function that doesn't, ends it.
    fn blamed(&self, span: Span) -> Span {
        let mut at = span;
        for &(call, blame) in self.calls.iter().rev() {
            match blame {
                Blame::No => break,
                Blame::Caller => at = call,
                Blame::Here => return call,
            }
        }
        at
    }

    /// A panic, which every function being run must say it may have: if
    /// one doesn't, the checker let through what it shouldn't have
    /// (see docs/07-effects.md). Not running out of stack, which no effect
    /// says.
    fn panics(&mut self, span: Span, message: impl Into<String>) -> Error {
        self.effect("panic");
        self.panicked = true;
        self.panic(span, message)
    }

    /// Running out of memory, which aborts as a panic does, but is part of
    /// `alloc`, not `panic`: every function being run must say `alloc`
    /// (see docs/16-out-of-memory.md).
    fn out_of_memory(&mut self, span: Span, message: impl Into<String>) -> Error {
        self.effect("alloc");
        self.panic(span, message)
    }

    /// Checks that every function being run says it may have effect
    /// `name`, as one that's happening. Closures are checked through the
    /// functions that call them, which have their effects.
    fn effect(&self, name: &str) {
        for f in &self.frames {
            let says = self.fns[f.as_str()]
                .effects
                .iter()
                .flat_map(|e| &e.items)
                .any(|e| e.to_string() == name);
            if !says {
                unsound(&format!(
                    "`{f}` doesn't say `with {name}`, but it's running at a `{name}`"
                ));
            }
        }
    }

    fn call(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        match f {
            // Glue, which monomorphisation wrote: a body with no
            // declaration, which nothing checks the effects of, as it has
            // none.
            Value::Fn(name, _) if is_glue(&name) => {
                let exe = self.exe;
                let (name, body) = (exe.body_named(&name))
                    .unwrap_or_else(|| unsound(&format!("glue `{name}` has no body")));
                self.run(body, name, None, args, Vec::new(), span)
            }
            Value::Fn(name, Some(at)) => {
                // Called through a value made at `at`: what aborts in it
                // is shown there, and nowhere further out, as the site is
                // the value's, not the call's.
                self.calls.push((at, Blame::Here));
                let result = self.call(Value::Fn(name, None), args, at);
                self.calls.pop();
                result
            }
            Value::Fn(name, None) => {
                let decl = self.fns[name.as_str()];
                // What a refinement names of the parameters: their values
                // here, which can't change.
                let env: Vec<(String, Value)> = decl
                    .params
                    .iter()
                    .zip(&args)
                    .map(|(p, v)| (p.name.name.clone(), v.clone()))
                    .collect();
                let refinement = |i: &Self, t: &TypeExpr| {
                    t.refinement()
                        .map(|(v, pred)| i.refined(&v.name, pred, env.clone()))
                };
                // A result's refinement and `ensures` are about what's
                // behind the arguments at return.
                let ret = (decl.ret.refinement())
                    .map(|(v, pred)| self.refined_at(&v.name, pred, env.clone(), true));
                let ensures: Vec<Refined> = (decl.ensures.iter())
                    .map(|e| self.refined_at("", e, env.clone(), true))
                    .collect();
                let mut refined = Vec::new();
                for (p, v) in decl.params.iter().zip(&args) {
                    let r = refinement(self, &p.ty);
                    if let Some(r) = &r {
                        r.check(
                            self,
                            v,
                            &format!("argument `{}` of `{}`", p.name.name, decl.name.name),
                        );
                    }
                    refined.push(r);
                }
                // What an intrinsic gives is trusted, and so checked: its
                // refinement too.
                let result = if decl.intrinsic.is_some() {
                    let of = name.split("::<").next().unwrap_or_default().to_string();
                    self.intrinsic(&of, &name, args, span)
                } else {
                    let exe = self.exe;
                    let (name, body) = (exe.body_named(&name))
                        .unwrap_or_else(|| unsound(&format!("`{name}` has no body")));
                    self.frames.push(name.to_string());
                    let result = self.run(body, name, Some(name), args, refined, span);
                    self.frames.pop();
                    result
                };
                if let (Ok(v), Some(r)) = (&result, ret) {
                    r.check(self, v, &format!("the result of `{}`", decl.name.name));
                }
                if result.is_ok() {
                    for r in &ensures {
                        let what = format!("what `{}` ensures", decl.name.name);
                        r.check(self, &Value::Unit, &what);
                    }
                }
                result
            }
            // A closure called by value is the call's to use up, and what's
            // left of it is dropped when it returns. One whose body takes it
            // by reference is called through one, by the shim
            // monomorphisation wrote instead (`Cx::by_ref_body`).
            Value::Closure(c) => {
                let exe = self.exe;
                let (code, body) = (exe.body_named(&c.code))
                    .unwrap_or_else(|| unsound(&format!("closure `{}` has no body", c.code)));
                let root = closure_root(code);
                match env_by(body) {
                    Some(None) => {
                        let mut all = vec![Value::Closure(c.clone())];
                        all.extend(args);
                        self.run(body, root, None, all, Vec::new(), span)
                    }
                    // One that captures nothing: there's nothing to it.
                    None => self.run(body, root, None, args, Vec::new(), span),
                    Some(Some(_)) => {
                        unsound("a closure whose body takes it by reference called by value")
                    }
                }
            }
            // Through a reference, what it points to is called, in place.
            Value::Ref(p) => match self.memory.get(&p).clone() {
                Value::Closure(_) => self.call_closure(&p, args, span),
                f => self.call(f, args, span),
            },
            other => unsound(&format!("called a non-function `{other}`")),
        }
    }

    /// Calls the closure at `at`, in place: its body gets a reference to it,
    /// as it's `Fn` or `FnMut`, or nothing, if it captures nothing.
    fn call_closure(&mut self, at: &Ptr, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        let Value::Closure(c) = self.memory.get(at).clone() else {
            unreachable!("a closure")
        };
        let exe = self.exe;
        let (code, body) = (exe.body_named(&c.code))
            .unwrap_or_else(|| unsound(&format!("closure `{}` has no body", c.code)));
        let all = match env_by(body) {
            Some(Some(_)) => std::iter::once(Value::Ref(at.clone()))
                .chain(args)
                .collect(),
            Some(None) => unsound("a closure that's `FnOnce` called through a reference"),
            None => args,
        };
        self.run(body, closure_root(code), None, all, Vec::new(), span)
    }

    /// Runs MIR `body`, of function `f` or else of a closure, both in the
    /// copy of a function `root`, with its parameters `args`, the first
    /// the closure if it's a closure's that captures, those with
    /// refinements having them, called at `span`. Each local's slot is made
    /// when it's first given a value, and freed when it goes out of scope
    /// (`StmtKind::Dead`), which drops what's in it.
    fn run(
        &mut self,
        body: &'p crate::mir::Body,
        root: &'p str,
        f: Option<&'p str>,
        args: Vec<Value>,
        refined: Vec<Option<Refined>>,
        span: Span,
    ) -> Result<Value, Error> {
        let blames = f.is_some_and(|f| self.fns.get(f).is_some_and(|d| d.blame_caller.is_some()));
        let blame = match blames {
            true => Blame::Caller,
            false => Blame::No,
        };
        self.calls.push((span, blame));
        let result = self.run_body(body, root, f, args, refined, span);
        self.calls.pop();
        result
    }

    /// `run`, once the call is on `calls`.
    fn run_body(
        &mut self,
        body: &'p crate::mir::Body,
        root: &'p str,
        f: Option<&'p str>,
        args: Vec<Value>,
        refined: Vec<Option<Refined>>,
        span: Span,
    ) -> Result<Value, Error> {
        use crate::mir::{Bb, StmtKind, Term};
        if self.depth >= MAX_CALL_DEPTH {
            return Err(self.panic(
                span,
                format!("stack overflow (more than {MAX_CALL_DEPTH} nested calls)"),
            ));
        }
        if args.len() != body.params {
            unsound("a call with the wrong number of arguments");
        }
        let mut fr = Frame {
            body,
            root,
            f,
            slots: vec![None; body.locals.len()],
        };
        // A function's parameters come after its closure, if it's a
        // closure's.
        let skip = args.len() - refined.len().min(args.len());
        for (i, v) in args.into_iter().enumerate() {
            let p = self.memory.alloc(v, false);
            if let Some(Some(r)) = i.checked_sub(skip).and_then(|j| refined.get(j)) {
                self.refined.insert((p.slot, p.generation), r.clone());
            }
            fr.slots[i + 1] = Some(p);
        }
        self.sites.push(Site {
            unsafe_blocks: &body.unsafe_blocks,
            span,
            f,
        });
        self.depth += 1;
        let mut b = Bb(0);
        let result = 'run: loop {
            let block = &body.blocks[b.0];
            for s in &block.stmts {
                self.sites.last_mut().expect("pushed").span = s.span;
                let done = match &s.kind {
                    StmtKind::Assign { place, value, .. } => {
                        match self.rvalue(&mut fr, value, s.span) {
                            Ok(v) => self.write(&mut fr, place, v),
                            Err(e) => Err(e),
                        }
                    }
                    StmtKind::Inspect(_) => Ok(()),
                    StmtKind::Dead(l) => self.dead(&mut fr, l.0),
                    // What the pattern didn't move out was dropped before.
                    StmtKind::Consume(place, _) => {
                        let p = self.ptr(&fr, place);
                        let old = std::mem::replace(self.memory.get_mut(&p), Value::Moved);
                        self.parts_left_behind(old, "taken apart by a pattern");
                        Ok(())
                    }
                    StmtKind::Drop(place, ty) => {
                        let p = self.ptr(&fr, place);
                        let v = std::mem::replace(self.memory.get_mut(&p), Value::Moved);
                        self.drop(v, ty, s.span)
                    }
                    StmtKind::Free(place) => {
                        let p = self.ptr(&fr, place);
                        match std::mem::replace(self.memory.get_mut(&p), Value::Moved) {
                            Value::Box(q) => {
                                let inner = self.memory.free(&q, true);
                                self.left_behind(&inner, "freed");
                                Ok(())
                            }
                            v => unsound(&format!("`free` of `{v}`, which isn't a box")),
                        }
                    }
                    StmtKind::Refine(l, r) => {
                        self.refine(&fr, *l, &body.refinements[*r]);
                        Ok(())
                    }
                    // The checker leaves only what it knew to be `Discard`,
                    // which a type parameter is only if every type it can
                    // be is.
                    StmtKind::Kept(place) => {
                        let p = self.ptr(&fr, place);
                        if let Value::Struct(ty, _) | Value::Variant(ty, ..) = self.memory.get(&p)
                            && !self.discard.contains(ty.as_str())
                        {
                            unsound(&format!(
                                "a `{}`, which isn't `Discard`, taken apart and not used up",
                                crate::check::source_name(ty)
                            ));
                        }
                        Ok(())
                    }
                };
                if let Err(e) = done {
                    break 'run Err(e);
                }
            }
            match block.term.as_ref() {
                Some(Term::Goto(t)) => b = *t,
                Some(Term::Branch(o, t, e)) => {
                    b = match bool(self.operand(&fr, o)) {
                        true => *t,
                        false => *e,
                    }
                }
                Some(Term::Match(place, pats, targets)) => {
                    let p = self.ptr(&fr, place);
                    let Some(i) = pats.iter().position(|pat| self.matches(pat, &p)) else {
                        unsound(&format!("no `match` arm fits `{}`", self.memory.get(&p)));
                    };
                    b = targets[i];
                }
                Some(Term::Return) => {
                    let v = match fr.slots[0].take() {
                        Some(p) => self.memory.free(&p, false),
                        None => unsound("a function returning with no value"),
                    };
                    if fr.slots.iter().any(Option::is_some) {
                        unsound("a local outlived the function it's in");
                    }
                    break Ok(v);
                }
                Some(Term::Unreachable) => unsound("control got where it can't"),
                None => unsound("a block with no end"),
            }
        };
        self.depth -= 1;
        self.sites.pop();
        result
    }

    /// Where the value at `place` is, in frame `fr`: following its
    /// fields and `*`s, which does nothing but read memory.
    fn ptr(&self, fr: &Frame, place: &crate::mir::Place) -> Ptr {
        use crate::mir::Proj;
        let Some(mut p) = fr.slots[place.local.0].clone() else {
            unsound("use of a local with no value")
        };
        for proj in &place.proj {
            p = match (proj, self.memory.get(&p)) {
                (Proj::Deref, Value::Ref(q) | Value::Box(q)) => q.clone(),
                (Proj::Deref, other) => unsound(&format!("`*` on `{other}`")),
                (Proj::Field(i), v @ Value::Struct(..)) => {
                    self.check_fields(v, "read");
                    p.field(*i)
                }
                (Proj::Field(i), Value::Closure(_) | Value::Slots(_)) => p.field(*i),
                (Proj::Variant(_, i), Value::Variant(..)) => p.field(*i),
                (_, Value::Moved) => unsound("use of a value that was moved"),
                (proj, other) => unsound(&format!("{proj:?} of `{other}`")),
            };
        }
        p
    }

    /// Value `v`, made at `span`: a function that blames its caller, or an
    /// intrinsic, is called there, wherever it's called through `v`
    /// (docs/34-blame-caller.md, B3).
    fn made_at(&self, v: Value, span: Span) -> Value {
        match v {
            Value::Fn(name, None) if self.blames_caller(&name) => Value::Fn(name, Some(span)),
            v => v,
        }
    }

    /// Whether function `name`'s aborts are shown where it's called: it
    /// says `#[blame_caller]`, or it's an intrinsic, which is called as an
    /// operator is.
    fn blames_caller(&self, name: &str) -> bool {
        (self.fns.get(name)).is_some_and(|d| d.blame_caller.is_some() || d.intrinsic.is_some())
    }

    /// What operand `o` gives, in frame `fr`: moved out of its place, or
    /// copied, as the borrow checker decided.
    fn operand(&mut self, fr: &Frame, o: &crate::mir::Operand) -> Value {
        use crate::mir::{Const, Operand};
        match o {
            Operand::Copy(p) => {
                let p = self.ptr(fr, p);
                self.read(&p, false)
            }
            Operand::Move(p) => {
                let p = self.ptr(fr, p);
                self.read(&p, true)
            }
            Operand::Const(c) => match c {
                Const::Int(n, crate::types::Type::Int(t)) => int_of(*n, *t),
                Const::Int(_, t) => unsound(&format!("an integer of type `{t}`")),
                Const::Bool(b) => Value::Bool(*b),
                Const::Char(c) => Value::Char(*c),
                Const::Unit => Value::Unit,
                Const::Str(s) => Value::Ref(self.literal(s.as_bytes())),
                Const::Bytes(b) => Value::Ref(self.literal(b)),
                Const::Fn(name, _) => Value::Fn(name.clone(), None),
                Const::Param(_) => unsound("a const parameter left after monomorphising"),
                Const::Receiver(_) => unsound("a call's receiver as a value"),
            },
        }
    }

    /// What `value` gives, in frame `fr`, at `span`.
    fn rvalue(
        &mut self,
        fr: &mut Frame,
        value: &crate::mir::Rvalue,
        span: Span,
    ) -> Result<Value, Error> {
        use crate::mir::{Agg, Const, Operand, Rvalue};
        use crate::types::Type;
        Ok(match value {
            Rvalue::Use(o) => {
                let v = self.operand(fr, o);
                self.made_at(v, span)
            }
            Rvalue::Ref(_, _, p) => Value::Ref(self.ptr(fr, p)),
            Rvalue::Unary(op, o, id) => {
                let v = self.operand(fr, o);
                match op {
                    UnOp::Neg => {
                        let zero = match v {
                            Value::Int(_, t) => Value::Int(0, t),
                            ref other => unsound(&format!("`-` of `{other}`")),
                        };
                        self.arith(BinOp::Sub, zero, v, *id, span)?
                    }
                    // Each bit flipped: the least plus the greatest, minus
                    // it, which is in its type, so never panics.
                    _ => match v {
                        Value::Int(n, t) => {
                            let (lo, hi) = t.range_64();
                            int_of(lo + hi - n, t)
                        }
                        v => Value::Bool(!bool(v)),
                    },
                }
            }
            // `==` looks at its operands where they are, and moves neither.
            Rvalue::Binary(op @ (BinOp::Eq | BinOp::Ne), l, r, _) => {
                let (Operand::Copy(l), Operand::Copy(r)) = (l, r) else {
                    unsound("`==` of what isn't a place")
                };
                let (l, r) = (self.ptr(fr, l), self.ptr(fr, r));
                let same = self.equal(&l, &r);
                Value::Bool(same == (*op == BinOp::Eq))
            }
            Rvalue::Binary(op, l, r, id) => {
                let l = self.operand(fr, l);
                let r = self.operand(fr, r);
                self.arith(*op, l, r, *id, span)?
            }
            Rvalue::Cast(o, t, id) => {
                let n = match self.operand(fr, o) {
                    Value::Char(c) => i128::from(u32::from(c)),
                    v => int(v),
                };
                match t {
                    // A character's scalar value; a number as a character,
                    // if it's one.
                    Type::Char => match u32::try_from(n).ok().and_then(char::from_u32) {
                        Some(c) => Value::Char(c),
                        None => {
                            self.not_proved(*id);
                            let message = format!("`{n}` isn't a character");
                            return Err(self.panics(span, message));
                        }
                    },
                    // The same number, if it fits.
                    Type::Int(t) => {
                        let (lo, hi) = t.range_64();
                        if !(lo..=hi).contains(&n) {
                            self.not_proved(*id);
                            let message = format!("`as` overflowed: {n} isn't {}", t.a());
                            return Err(self.panics(span, message));
                        }
                        Value::Int(n, *t)
                    }
                    t => unsound(&format!("`as` to `{t}`")),
                }
            }
            Rvalue::Aggregate(k, t, os) => {
                let values: Vec<Value> = (os.iter())
                    .map(|o| {
                        let v = self.operand(fr, o);
                        self.made_at(v, span)
                    })
                    .collect();
                match (k, t) {
                    (Agg::Struct, Type::Adt(name, _, args)) => {
                        let name = crate::mono::mangle(name, args);
                        let decl = self.structs[name.as_str()];
                        let fields = (decl.fields.iter())
                            .zip(values)
                            .map(|(f, v)| (f.name.name.clone(), v))
                            .collect();
                        let v = Value::Struct(name, fields);
                        self.check_fields(&v, "made");
                        v
                    }
                    (Agg::Variant(i), Type::Adt(name, _, args)) => {
                        let name = crate::mono::mangle(name, args);
                        let variant = self.enums[name.as_str()].variants[*i].name.name.clone();
                        Value::Variant(name, variant, values)
                    }
                    (Agg::Box, _) => {
                        let [v] = <[Value; 1]>::try_from(values)
                            .unwrap_or_else(|_| unsound("a box of one value"));
                        self.effect("alloc");
                        Value::Box(self.memory.alloc(v, true))
                    }
                    (Agg::Array, _) => Value::Slots(values),
                    // `n` copies of `v`, which is `Copy`.
                    (Agg::Repeat, Type::Array(_, n)) => {
                        let [v] = <[Value; 1]>::try_from(values)
                            .unwrap_or_else(|_| unsound("a repeat of one value"));
                        let Type::Nat(n) = **n else {
                            unsound("an array of no length")
                        };
                        Value::Slots(vec![v; n as usize])
                    }
                    // What it captured, by the names its body knows them by.
                    (Agg::Closure(id), _) => {
                        let code = crate::mono::closure_copy(fr.root, *id);
                        let exe = self.exe;
                        let body = (exe.body(&code))
                            .unwrap_or_else(|| unsound(&format!("closure `{code}` has no body")));
                        let env = (body.env.iter().map(|v| v.name.clone()))
                            .zip(values)
                            .collect();
                        Value::Closure(Closure { code, env })
                    }
                    (k, t) => unsound(&format!("{k:?} of type `{t}`")),
                }
            }
            Rvalue::Call(f, args) => {
                let mut args: Vec<Value> = (args.iter())
                    .map(|o| {
                        let v = self.operand(fr, o);
                        self.made_at(v, span)
                    })
                    .collect();
                let f = match f {
                    Operand::Const(Const::Receiver(_)) => {
                        if args.is_empty() {
                            unsound("a call of a receiver with none");
                        }
                        args.remove(0)
                    }
                    f => self.operand(fr, f),
                };
                self.call(f, args, span)?
            }
        })
    }

    /// Writes `v` to `place`, in frame `fr`: what was there is dropped. A
    /// local with no slot yet gets one.
    fn write(&mut self, fr: &mut Frame, place: &crate::mir::Place, v: Value) -> Result<(), Error> {
        if place.proj.is_empty() && fr.slots[place.local.0].is_none() {
            fr.slots[place.local.0] = Some(self.memory.alloc(v, false));
            return Ok(());
        }
        let p = self.ptr(fr, place);
        if p.path.is_empty()
            && let Some(r) = self.refined.get(&(p.slot, p.generation))
        {
            r.check(self, &v, "a refined variable assigned");
        }
        let old = std::mem::replace(self.memory.get_mut(&p), v);
        self.left_behind(&old, "assigned over");
        if !p.path.is_empty() {
            let mut outer = p.clone();
            outer.path.pop();
            self.check_fields(self.memory.get(&outer), "assigned to");
        }
        Ok(())
    }

    /// Local `l` of frame `fr` goes out of scope: what's in its slot is
    /// dropped, and the slot freed.
    ///
    /// In a type's own `discard`, `self` is dropped part by part, as the
    /// borrow checker has it (lemma D3 of docs/20-destructors.md): its
    /// `discard` is what's running.
    fn dead(&mut self, fr: &mut Frame, l: usize) -> Result<(), Error> {
        let Some(p) = fr.slots[l].take() else {
            return Ok(());
        };
        let v = self.memory.free(&p, false);
        match fr.f.is_some_and(is_discard_fn) && l == 1 {
            true => self.parts_left_behind(v, "dropped by its own `discard`"),
            false => self.left_behind(&v, "out of scope"),
        }
        Ok(())
    }

    /// `left_behind` of each part of `v`, which was taken apart, or in its
    /// own `discard`, is being dropped part by part.
    fn parts_left_behind(&self, v: Value, what: &str) {
        let parts: Vec<Value> = match v {
            Value::Struct(_, fields) => fields.into_iter().map(|(_, v)| v).collect(),
            Value::Variant(_, _, fields) => fields,
            Value::Slots(values) => values,
            Value::Moved => Vec::new(),
            v => unsound(&format!("`{v}` taken apart")),
        };
        for v in &parts {
            self.left_behind(v, what);
        }
    }

    /// Stops the program, as drop elaboration is wrong, if `v`, just
    /// `what`, holds anything dropping it would have done something
    /// with: elaboration should have dropped it first (E1 of
    /// docs/32.1-drops.md).
    fn left_behind(&self, v: &Value, what: &str) {
        if self.needs_drop(v) {
            unsound(&format!(
                "`{v}` was {what} still holding what needed dropping: drop elaboration is wrong"
            ));
        }
    }

    /// Would dropping `v` do anything: free memory, call a `discard`, or
    /// throw away what isn't `Discard`?
    fn needs_drop(&self, v: &Value) -> bool {
        match v {
            Value::Box(_) | Value::RawBuf(_) | Value::Arena(_) => true,
            Value::Struct(ty, _) | Value::Variant(ty, ..)
                if !self.taken_apart(v)
                    && (!self.discard.contains(ty.as_str()) || self.user_discard(ty).is_some()) =>
            {
                true
            }
            Value::Struct(_, fields) => fields.iter().any(|(_, v)| self.needs_drop(v)),
            Value::Variant(_, _, fields) => fields.iter().any(|v| self.needs_drop(v)),
            Value::Closure(c) => c.env.iter().any(|(_, v)| self.needs_drop(v)),
            Value::Slots(values) => values.iter().any(|v| self.needs_drop(v)),
            _ => false,
        }
    }

    /// Checks that local `l` of frame `fr`, just given its value, has
    /// refinement `r`, and keeps it, for what's assigned to it.
    fn refine(&mut self, fr: &Frame, l: crate::mir::Local, r: &crate::mir::Refinement) {
        let env = (r.env.iter())
            .map(|(n, place)| {
                let p = self.ptr(fr, place);
                (n.clone(), self.memory.get(&p).clone())
            })
            .collect();
        let refined = self.refined(&r.binder, &r.pred, env);
        let p = self.ptr(fr, &crate::mir::Place::local(l));
        let name = fr.body.locals[l.0].name.as_deref().unwrap_or("_");
        refined.check(self, self.memory.get(&p), &format!("`{name}`"));
        self.refined.insert((p.slot, p.generation), refined);
    }

    /// Does the value at `v` match `pat`?
    fn matches(&self, pat: &crate::mir::Pat, v: &Ptr) -> bool {
        use crate::mir::Pat;
        let in_range = |lo: &Option<i128>, hi: &Option<i128>, m: i128| {
            lo.is_none_or(|lo| lo <= m) && hi.is_none_or(|hi| m <= hi)
        };
        match (pat, self.memory.get(v)) {
            (Pat::Any, _) => true,
            (Pat::Deref(pat), Value::Ref(q)) => self.matches(pat, q),
            (Pat::Range(lo, hi), Value::Int(m, _)) => in_range(lo, hi, *m),
            (Pat::Range(lo, hi), Value::Char(c)) => in_range(lo, hi, i128::from(u32::from(*c))),
            (Pat::Bool(b), Value::Bool(c)) => b == c,
            (Pat::Variant(i, pats), Value::Variant(ty, name, _)) => {
                self.enums[ty.as_str()].variants[*i].name.name == *name
                    && (pats.iter().enumerate()).all(|(j, p)| self.matches(p, &v.field(j)))
            }
            (Pat::Struct(fields), Value::Struct(..)) => {
                fields.iter().all(|(j, p)| self.matches(p, &v.field(*j)))
            }
            (Pat::Array(pats), Value::Slots(values)) => {
                if pats.len() != values.len() {
                    unsound("an array pattern of another length");
                }
                (pats.iter().enumerate()).all(|(j, p)| self.matches(p, &v.field(j)))
            }
            (Pat::Or(alts), _) => alts.iter().any(|alt| self.matches(alt, v)),
            (_, Value::Moved) => unsound("matching a value that was moved"),
            (pat, v) => unsound(&format!("pattern {pat:?} can't apply to `{v}`")),
        }
    }
    /// Runs intrinsic `name` (see `check::INTRINSICS`), whose copy is
    /// `copy`, on `args`, called at `span`.
    fn intrinsic(
        &mut self,
        name: &str,
        copy: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Error> {
        if name == "core::num::checked_mul" {
            let [a, b] = <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("checked arity"));
            let (Value::Int(a, t), Value::Int(b, u)) = (a, b) else {
                unsound("`checked_mul` of what isn't an integer")
            };
            if t != u {
                unsound("`checked_mul` of two integer types");
            }
            let (lo, hi) = t.range_64();
            let result = a.checked_mul(b).filter(|n| (lo..=hi).contains(n));
            // The copy of `Option<T>` the intrinsic returns.
            let TypeExprKind::Named(option, _) = &self.fns[copy].ret.kind else {
                unsound("`checked_mul` returns an `Option<T>`")
            };
            return Ok(match result {
                Some(n) => Value::Variant(option.clone(), "Some".into(), vec![Value::Int(n, t)]),
                None => Value::Variant(option.clone(), "None".into(), Vec::new()),
            });
        }
        if name == "core::num::wrapping_mul" {
            let [a, b] = <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("checked arity"));
            let (Value::Int(a, t), Value::Int(b, u)) = (a, b) else {
                unsound("`wrapping_mul` of what isn't an integer")
            };
            if t != u {
                unsound("`wrapping_mul` of two integer types");
            }
            return Ok(int_of(wrap(a.wrapping_mul(b), t), t));
        }
        if name == "core::num::wrap" {
            let [Value::Int(x, _)] =
                <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("checked arity"))
            else {
                unsound("`wrap` of what isn't an integer")
            };
            let u = int_ty(&self.fns[copy].ret);
            return Ok(int_of(wrap(x, u), u));
        }
        if let Some(end) = name.strip_prefix("core::num::") {
            let t = int_ty(&self.fns[copy].ret);
            let (lo, hi) = t.range_64();
            return Ok(match end {
                "least" => Value::Int(lo, t),
                "greatest" => Value::Int(hi, t),
                _ => unsound(&format!("unknown intrinsic `{name}`")),
            });
        }
        if let Some(op) = name.strip_prefix("<core::panic::Message>::") {
            return self.message(op, args, span);
        }
        if let Some(stream) = name.strip_prefix("<std::io::") {
            return Ok(self.stream(stream, args));
        }
        if name == "<alloc::arena::Arena" {
            let op = copy.rsplit("::").next().unwrap_or_default();
            return Ok(self.arena(op, args));
        }
        if name == "<alloc::raw::RawBuf" {
            let op = copy.rsplit("::").next().unwrap_or_default();
            return self.raw_buf(op, args, span);
        }
        // A string is its bytes.
        if name == "str::as_bytes" || name == "<str>::as_bytes" {
            let [s] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
            return Ok(s);
        }
        if name == "str::len" || name == "<str>::len" {
            return Ok(self.slice("len", copy, args));
        }
        if name.starts_with("<[") {
            let op = copy.rsplit("::").next().unwrap_or_default();
            return Ok(self.slice(op, copy, args));
        }
        match name {
            // No slots: of any type, as there's nothing there.
            "core::slice::empty" | "core::slice::empty_mut" => {
                Ok(Value::Ref(self.nowhere.slice(0, 0)))
            }
            "alloc::raw::out_of_memory" => Err(self.out_of_memory(span, "out of memory")),
            // `f(0)`, `f(1)`, .., each made in place, as many as the copy's
            // array type says.
            // `f(0)`, `f(1)`, .., each made in place, as many as the copy's
            // array type says, calling `f` where it is. On a panic, what's
            // made is forgotten: the program is aborting.
            "core::array::from_fn_mut" => {
                let [f] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
                let n = match &self.fns[copy].ret.kind {
                    TypeExprKind::Array(_, n) => match n.kind {
                        TypeExprKind::Nat(n) => n as usize,
                        _ => unsound("`from_fn` of an array of no length"),
                    },
                    _ => unsound("`from_fn` makes an array"),
                };
                let mut values = Vec::new();
                for i in 0..n {
                    values.push(self.call(f.clone(), vec![usize_of(i)], span)?);
                }
                Ok(Value::Slots(values))
            }
            // Aborts, saying why.
            "core::panic::panic" => {
                let [why] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("panic arity"));
                let why = self.text(&why);
                Err(self.panics(span, format!("panicked: {why}")))
            }
            // The same bytes: a `str`'s are UTF-8, which is what it adds.
            "core::str::from_utf8_unchecked" => {
                let [b] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
                let block = self.unsafe_op();
                if std::str::from_utf8(&self.bytes(&b)).is_err() {
                    let why = "`str::from_utf8_unchecked` of bytes that aren't UTF-8".to_string();
                    return Err(self.broken(span, block, why));
                }
                Ok(b)
            }
            // Exchanges two values, making and throwing away none.
            "core::mem::swap" => {
                let [a, b] = <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("swap arity"));
                let (Value::Ref(p), Value::Ref(q)) = (&a, &b) else {
                    unsound(&format!("`swap` of `{a}` and `{b}`"))
                };
                let x = std::mem::replace(self.memory.get_mut(p), Value::Moved);
                let y = std::mem::replace(self.memory.get_mut(q), x);
                if let Value::Moved = y {
                    unsound("`swap` of a value that was moved, or of a place with itself");
                }
                *self.memory.get_mut(p) = y;
                Ok(Value::Unit)
            }
            _ => unsound(&format!("unknown intrinsic `{name}`")),
        }
    }

    /// The intrinsic method `op` of `core::panic::Message`, the message
    /// `panic_with` writes, with arguments `args`. There's only ever one
    /// being written: `fmt` into it can't panic, so can't start another.
    fn message(&mut self, op: &str, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        match (op, args.as_slice()) {
            ("new", []) => {
                self.message.clear();
                Ok(Value::Struct("core::panic::Message".into(), Vec::new()))
            }
            ("push_str", [_, s]) => {
                let s = self.text(s);
                self.message.push_str(&s);
                Ok(Value::Unit)
            }
            ("push_char", [_, Value::Char(c)]) => {
                self.message.push(*c);
                Ok(Value::Unit)
            }
            ("abort", [_]) => {
                let why = std::mem::take(&mut self.message);
                Err(self.panics(span, format!("panicked: {why}")))
            }
            _ => unsound(&format!("unknown intrinsic `<core::panic::Message>::{op}`")),
        }
    }

    /// The intrinsic method of `std::io`'s stream, `stream` being its
    /// copy's name after `<std::io::`, with arguments `args`.
    fn stream(&mut self, stream: &str, args: Vec<Value>) -> Value {
        let code = match (stream, args.as_slice()) {
            ("Stdin>::read_byte", [_]) => match self.term.read_byte() {
                Ok(Some(b)) => u32::from(b),
                Ok(None) => 256,
                Err(e) => io_error(&e),
            },
            ("Stdout>::write_byte" | "Stderr>::write_byte", [_, b]) => {
                let b = u8::try_from(int(b.clone())).unwrap_or_else(|_| unsound("a byte"));
                match self.term.write_byte(stream.starts_with("Stderr"), b) {
                    Ok(()) => 0,
                    Err(e) => io_error(&e),
                }
            }
            _ => unsound(&format!("unknown intrinsic `<std::io::{stream}`")),
        };
        Value::Int(i128::from(code), IntTy::U32)
    }

    /// Where the literal of bytes `bytes` is: a slice of `u8`s, which a
    /// string literal is too.
    fn literal(&mut self, bytes: &[u8]) -> Ptr {
        if let Some(p) = self.literals.get(bytes) {
            return p.clone();
        }
        let slots = bytes
            .iter()
            .map(|&b| Value::Int(i128::from(b), IntTy::U8))
            .collect();
        let p = self
            .memory
            .alloc(Value::Slots(slots), false)
            .slice(0, bytes.len());
        self.literals.insert(bytes.to_vec(), p.clone());
        p
    }

    /// The bytes of the slice of `u8`s `v` is a reference to: a `&[u8]` or
    /// a `&str`.
    fn bytes(&self, v: &Value) -> Vec<u8> {
        let Value::Ref(
            p @ Ptr {
                range: Some((lo, len)),
                ..
            },
        ) = v
        else {
            unsound(&format!("`{v}` as a slice of bytes"))
        };
        let Value::Slots(slots) = self.memory.get(p) else {
            unsound("a slice that isn't of slots")
        };
        slots[*lo..lo + len]
            .iter()
            .map(|v| match v {
                Value::Int(n, IntTy::U8) => *n as u8,
                v => unsound(&format!("`{v}` as a byte")),
            })
            .collect()
    }

    /// The text of string `v`, a `&str`, which is UTF-8.
    fn text(&self, v: &Value) -> String {
        String::from_utf8(self.bytes(v)).unwrap_or_else(|_| unsound("a `str` that isn't UTF-8"))
    }

    /// Runs a slice's intrinsic method `op`, whose copy is `copy`, on
    /// `args`. Each checks what it's given against the slice's length, so
    /// none can fail: it's what it says in `core/slice.t`.
    fn slice(&mut self, op: &str, copy: &str, args: Vec<Value>) -> Value {
        let mut args = args.into_iter();
        let this = args.next().unwrap_or_else(|| unsound("slice arity"));
        // An array's: a reference to its values, all of them.
        if let Value::Ref(p @ Ptr { range: None, .. }) = &this {
            let Value::Slots(values) = self.memory.get(p) else {
                unsound(&format!("`{op}` of `{this}`, which isn't an array"))
            };
            let len = values.len();
            return match op {
                "len" => usize_of(len),
                "as_slice" | "as_mut_slice" => Value::Ref(p.slice(0, len)),
                _ => unsound(&format!("unknown intrinsic `[T; N]::{op}`")),
            };
        }
        let Value::Ref(
            p @ Ptr {
                range: Some((lo, len)),
                ..
            },
        ) = &this
        else {
            unsound(&format!("`{op}` of `{this}`, which isn't a slice"))
        };
        let (p, lo, len) = (p.clone(), *lo, *len);
        // The copy of `Option` it returns, and what's in its `Some`.
        let option = || -> (String, Option<String>) {
            let TypeExprKind::Named(option, _) = &self.fns[copy].ret.kind else {
                unsound(&format!("`{op}` returns an `Option`"))
            };
            let some = self.enums[option.as_str()]
                .variants
                .iter()
                .find(|v| v.name.name == "Some")
                .and_then(|v| match &v.fields[0].kind {
                    TypeExprKind::Named(name, _) => Some(name.clone()),
                    _ => None,
                });
            (option.clone(), some)
        };
        let index = |v: Option<Value>, max: usize| {
            let i = int(v.unwrap_or_else(|| unsound("slice arity")));
            usize::try_from(i).ok().filter(|i| *i <= max)
        };
        let made = match op {
            "len" => return usize_of(len),
            // Its refinement says `i < len()`, so it's the checker's bug
            // if it isn't.
            "index" | "index_mut" => {
                let Some(i) = index(args.next(), len).filter(|i| *i < len) else {
                    unsound(&format!("`{op}` out of range of a slice of {len}"))
                };
                return Value::Ref(Ptr {
                    range: None,
                    ..p.field(lo + i)
                });
            }
            "get" | "get_mut" => {
                let i = index(args.next(), len).filter(|i| *i < len);
                i.map(|i| {
                    Value::Ref(Ptr {
                        range: None,
                        ..p.field(lo + i)
                    })
                })
            }
            "split_at" | "split_at_mut" => index(args.next(), len).map(|mid| {
                let split = option().1.unwrap_or_else(|| unsound("a split's struct"));
                let head = Value::Ref(p.slice(lo, mid));
                let tail = Value::Ref(p.slice(lo + mid, len - mid));
                Value::Struct(split, vec![("head".into(), head), ("tail".into(), tail)])
            }),
            _ => unsound(&format!("unknown intrinsic `[T]::{op}`")),
        };
        let option = option().0;
        match made {
            Some(v) => Value::Variant(option, "Some".into(), vec![v]),
            None => Value::Variant(option, "None".into(), Vec::new()),
        }
    }

    /// Runs `Arena`'s intrinsic method `op` on `args`. Each value put in
    /// gets a heap slot of its own, which nothing frees before the arena
    /// is dropped, so a reference `alloc` hands out stays good until then
    /// (lemmas R1 and R2 of docs/23-arenas.md), and dangles after, which
    /// the generations of slots catch.
    fn arena(&mut self, op: &str, args: Vec<Value>) -> Value {
        // Where the boxes of the arena that `self`, a `&Arena`, points to are.
        let boxes = |this: &Interp, v: &Value| -> Ptr {
            let Value::Ref(p) = v else {
                unsound(&format!("`Arena::{op}` of `{v}`"))
            };
            match this.memory.get(p) {
                Value::Arena(q) => q.clone(),
                other => unsound(&format!("`Arena::{op}` of `{other}`")),
            }
        };
        let mut args = args.into_iter();
        let mut arg = || args.next().unwrap_or_else(|| unsound("`Arena` arity"));
        match op {
            "new" => Value::Arena(self.memory.alloc(Value::Slots(Vec::new()), true)),
            "alloc" => {
                let q = boxes(self, &arg());
                let v = arg();
                self.effect("alloc");
                let p = self.memory.alloc(v, true);
                match self.memory.get_mut(&q) {
                    Value::Slots(values) => values.push(Value::Box(p.clone())),
                    other => unsound(&format!("an `Arena` of `{other}`")),
                }
                Value::Ref(p)
            }
            "len" => {
                let q = boxes(self, &arg());
                match self.memory.get(&q) {
                    Value::Slots(values) => usize_of(values.len()),
                    other => unsound(&format!("an `Arena` of `{other}`")),
                }
            }
            _ => unsound(&format!("unknown intrinsic `Arena::{op}`")),
        }
    }

    /// Runs `RawBuf`'s intrinsic method `op` on `args`, called at `span`.
    /// Its slots are a `Slots` on the heap, so a reference to one is into
    /// that, and `grow` and `free` leave it dangling, as they would a
    /// compiled program's. Each unsafe one checks its obligations, as
    /// `alloc/raw.t` says them, and stops the program if they're broken,
    /// blaming the `unsafe` block that said they were kept.
    fn raw_buf(&mut self, op: &str, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        /// More slots than this is running out of memory.
        const MAX_SLOTS: i128 = 1 << 20;
        // The buffer that `self`, a `&RawBuf` or `&mut RawBuf`, points to.
        let buf = |this: &Interp, v: &Value| -> Ptr {
            let Value::Ref(p) = v else {
                unsound(&format!("`RawBuf::{op}` of `{v}`"))
            };
            match this.memory.get(p) {
                Value::RawBuf(q) => q.clone(),
                other => unsound(&format!("`RawBuf::{op}` of `{other}`")),
            }
        };
        let slots = |this: &Interp, q: &Ptr| -> usize {
            match this.memory.get(q) {
                Value::Slots(slots) => slots.len(),
                other => unsound(&format!("a `RawBuf` of `{other}`")),
            }
        };
        let mut args = args.into_iter();
        let mut arg = || args.next().unwrap_or_else(|| unsound("`RawBuf` arity"));
        match op {
            "new" => {
                let cap = int(arg());
                self.effect("alloc");
                if cap > MAX_SLOTS {
                    return Err(self.out_of_memory(span, format!("out of memory for {cap} slots")));
                }
                let slots = vec![Value::Moved; usize::try_from(cap).unwrap_or(0)];
                Ok(Value::RawBuf(self.memory.alloc(Value::Slots(slots), true)))
            }
            // No slots, so nothing to allocate: a compiled program's would
            // be a dangling pointer. It's on the heap here only so that
            // the leak check sees it freed, as every buffer must be.
            "empty" => Ok(Value::RawBuf(
                self.memory.alloc(Value::Slots(Vec::new()), true),
            )),
            "cap" => {
                let q = buf(self, &arg());
                Ok(usize_of(slots(self, &q)))
            }
            "grow" => {
                let this = arg();
                let (q, extra) = (buf(self, &this), int(arg()));
                // Both are `usize`s, so their sum fits in an `i128`.
                let new_cap = slots(self, &q) as i128 + extra;
                self.effect("alloc");
                if new_cap > MAX_SLOTS {
                    let message = format!("out of memory for {new_cap} slots");
                    return Err(self.out_of_memory(span, message));
                }
                // Somewhere new, so that what pointed into the old slots
                // dangles.
                let Value::Slots(mut slots) = self.memory.free(&q, true) else {
                    unreachable!("checked by `slots`")
                };
                slots.resize(new_cap as usize, Value::Moved);
                let moved = self.memory.alloc(Value::Slots(slots), true);
                let Value::Ref(p) = this else {
                    unreachable!("checked by `buf`")
                };
                *self.memory.get_mut(&p) = Value::RawBuf(moved);
                Ok(Value::Unit)
            }
            "slice" | "slice_mut" => {
                let block = self.unsafe_op();
                let q = buf(self, &arg());
                let (lo, hi) = (int(arg()), int(arg()));
                let cap = slots(self, &q);
                let what = format!("`RawBuf::{op}` of slots {lo}..{hi}");
                // That half is its type's, `{ l: i64 | l >= 0 }`.
                if lo < 0 {
                    unsound(&format!("{what}, but {lo} isn't `>= 0`"));
                }
                if hi > cap as i128 {
                    let why = format!("{what}, but the buffer has {cap}");
                    return Err(self.broken(span, block, why));
                }
                if lo > hi {
                    let why = format!("{what}, which is no range: {lo} > {hi}");
                    return Err(self.broken(span, block, why));
                }
                let (lo, hi) = (lo as usize, hi as usize);
                if let Some(i) =
                    (lo..hi).find(|i| matches!(self.memory.get(&q.field(*i)), Value::Moved))
                {
                    let why = format!("{what}, but slot {i} is uninitialised");
                    return Err(self.broken(span, block, why));
                }
                Ok(Value::Ref(q.slice(lo, hi - lo)))
            }
            "write" | "read" => {
                let block = self.unsafe_op();
                let this = arg();
                let (q, i) = (buf(self, &this), int(arg()));
                let cap = slots(self, &q);
                let what = format!("`RawBuf::{op}` of slot {i}");
                // That half is its type's, `{ i: i64 | i >= 0 }`.
                if i < 0 {
                    unsound(&format!("{what}, which isn't `>= 0`"));
                }
                if i >= cap as i128 {
                    let why = format!("{what}, but it has {cap} slots");
                    return Err(self.broken(span, block, why));
                }
                let slot = q.field(i as usize);
                let filled = !matches!(self.memory.get(&slot), Value::Moved);
                match (op, filled) {
                    ("write", true) => {
                        Err(self.broken(span, block, format!("{what}, which is initialised")))
                    }
                    ("write", false) => {
                        *self.memory.get_mut(&slot) = arg();
                        Ok(Value::Unit)
                    }
                    (_, false) => {
                        Err(self.broken(span, block, format!("{what}, which is uninitialised")))
                    }
                    _ => Ok(std::mem::replace(self.memory.get_mut(&slot), Value::Moved)),
                }
            }
            "free" => {
                let block = self.unsafe_op();
                let Value::RawBuf(q) = arg() else {
                    unsound("`RawBuf::free` of something else")
                };
                let Value::Slots(slots) = self.memory.get(&q) else {
                    unsound("a `RawBuf` of something else")
                };
                if let Some(i) = slots.iter().position(|v| !matches!(v, Value::Moved)) {
                    let why = format!("`RawBuf::free` with slot {i} initialised");
                    return Err(self.broken(span, block, why));
                }
                self.memory.free(&q, true);
                Ok(Value::Unit)
            }
            _ => unsound(&format!("unknown intrinsic `RawBuf::{op}`")),
        }
    }

    /// The innermost `unsafe` block being run, at an unsafe operation:
    /// there must be one, around where the function or closure calling it
    /// is, or where one that called it is, and every function on the way
    /// must say `with unsafe`, or the checker let through what it shouldn't
    /// have (see docs/09-unsafe.md).
    fn unsafe_op(&self) -> Span {
        for site in self.sites.iter().rev() {
            let inside = |b: &&Span| b.start <= site.span.start && site.span.end <= b.end;
            if let Some(block) = site.unsafe_blocks.iter().find(inside) {
                return *block;
            }
            let Some(f) = site.f else { continue };
            let says = self.fns[f]
                .effects
                .iter()
                .flat_map(|e| &e.items)
                .any(|e| e.to_string() == "unsafe");
            if !says {
                unsound(&format!(
                    "`{f}` doesn't say `with unsafe`, but it's running an unsafe operation"
                ));
            }
        }
        unsound("an unsafe operation with no `unsafe` block around it")
    }

    /// The error of an unsafe operation at `span` whose obligation is
    /// broken, `why`, in `unsafe` block `block`. Not a soundness bug of the
    /// checker's, but of the code in the block: it stops the program, as a
    /// panic does.
    fn broken(&mut self, span: Span, block: Span, why: String) -> Error {
        self.panic(span, format!("unsafe code broke its obligation: {why}"))
            .with_note(
                block,
                "in this `unsafe` block, which says its obligations are kept",
            )
    }

    /// Stops the program, as the checker is wrong, unless struct value
    /// `v`, just `what`, has the refinements of its fields.
    fn check_fields(&self, v: &Value, what: &str) {
        let Value::Struct(name, fields) = v else {
            return;
        };
        let Some(decl) = self.structs.get(name.as_str()) else {
            return;
        };
        for (j, f) in decl.fields.iter().enumerate() {
            if let Some((binder, pred)) = f.ty.refinement() {
                let r = self.refined(&binder.name, pred, fields[..j].to_vec());
                r.check(
                    self,
                    &fields[j].1,
                    &format!("field `{}` of a `{name}` {what}", f.name.name),
                );
            }
        }
    }

    /// `==` on the values at `p` and `q`: numbers, `bool`s, characters or
    /// `()`. Monomorphisation made every other `==` a call of what compares
    /// it, a type's own `eq` or its glue (see `mono/glue.rs`).
    fn equal(&self, p: &Ptr, q: &Ptr) -> bool {
        match (self.memory.get(p), self.memory.get(q)) {
            (a @ (Value::Int(..) | Value::Bool(_) | Value::Char(_) | Value::Unit), b) => a == b,
            (a, b) => unsound(&format!(
                "`==` of `{a}` and `{b}`, left after monomorphising"
            )),
        }
    }

    /// Stops the program, as the checker is wrong, if it proved the
    /// operator at `id` can't panic: it's about to.
    fn not_proved(&self, id: NodeId) {
        if self.proved.contains(&id) {
            unsound("an operator the checker proved can't panic is panicking");
        }
    }

    /// `l op r`, for the operator `e`: in ℤ, as long as the result fits in
    /// their type.
    fn arith(
        &mut self,
        op: BinOp,
        l: Value,
        r: Value,
        id: NodeId,
        span: Span,
    ) -> Result<Value, Error> {
        let (Value::Int(l, t), Value::Int(r, u)) = (&l, &r) else {
            unsound(&format!("`{op}` of `{l}` and `{r}`"))
        };
        let shift = matches!(op, BinOp::Shl | BinOp::Shr);
        if shift && *u != IntTy::U32 {
            unsound(&format!("`{op}` by {}", u.a()));
        }
        if !shift && t != u {
            unsound(&format!("`{op}` of {} and {}", t.a(), u.a()));
        }
        let (l, r, t) = (*l, *r, *t);
        let (lo, hi) = t.range_64();
        let n = match op {
            // Bit by bit, which `i128`'s operators are, on two's
            // complement: the result is in `t` (`int_of` checks).
            BinOp::BitAnd => return Ok(int_of(l & r, t)),
            BinOp::BitOr => return Ok(int_of(l | r, t)),
            BinOp::BitXor => return Ok(int_of(l ^ r, t)),
            // Rounding down, as `i128`'s `>>` does. Past 126 bits, every
            // integer of any type is 0, or -1 if it's negative.
            BinOp::Shr => return Ok(int_of(l >> r.min(127), t)),
            // Times 2^r, which overflows `i128` past 126 bits but for 0.
            BinOp::Shl if l == 0 => Some(0),
            BinOp::Shl if r >= 127 => None,
            BinOp::Shl => l.checked_mul(1 << r),
            BinOp::Lt => return Ok(Value::Bool(l < r)),
            BinOp::Le => return Ok(Value::Bool(l <= r)),
            BinOp::Gt => return Ok(Value::Bool(l > r)),
            BinOp::Ge => return Ok(Value::Bool(l >= r)),
            BinOp::Div | BinOp::Rem if r == 0 => {
                self.not_proved(id);
                return Err(self.panics(span, "division by zero"));
            }
            BinOp::Add => l.checked_add(r),
            BinOp::Sub => l.checked_sub(r),
            BinOp::Mul => l.checked_mul(r),
            BinOp::Div => l.checked_div(r),
            // Its result would fit, as it's 0, but it's an overflow on a
            // machine, which divides to find it.
            BinOp::Rem if l == lo && r == -1 => None,
            BinOp::Rem => l.checked_rem(r),
            BinOp::Eq | BinOp::Ne | BinOp::And | BinOp::Or => unreachable!("handled in eval"),
        };
        match n.filter(|n| (lo..=hi).contains(n)) {
            Some(n) => Ok(Value::Int(n, t)),
            None => {
                self.not_proved(id);
                Err(self.panics(span, "integer overflow"))
            }
        }
    }
}
