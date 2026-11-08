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
use crate::intrinsic::{
    ArgsOp, ArrayOp, AtomicOp, ClockOp, EnvOp, FsOp, Intrinsic, MessageOp, Output, OutputOp,
    RawBufOp, ReadFileOp, SliceOp, SlotsOp, StdinOp, VolatileOp,
};
use crate::mono::{glue_name, is_glue};
use crate::types::IntTy;
use std::collections::{HashMap, HashSet};
use std::fmt;

mod threads;

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
    /// A `JoinHandle`: which thread it's of, by number (see `threads`).
    Thread(usize),
    /// An arena's `Slots`: where its values are, on the heap, as a `Slots`
    /// of a `Box` for each, in the order they were put in.
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

/// How the body of a closure takes the closure: `Some(m)` by reference,
/// `&mut` if `m`, as it's `Fn` or `FnMut`; `None` by value, as it's
/// `FnOnce`.
pub(crate) fn env_by(body: &crate::mir::Body) -> Option<bool> {
    let env = (body.locals.get(1))
        .filter(|l| l.kind == crate::mir::LocalKind::Env)
        .unwrap_or_else(|| unsound("a closure's body that doesn't take the closure"));
    match &env.ty {
        crate::types::Type::Ref(_, m, _) => Some(*m),
        _ => None,
    }
}

/// A function or closure being run: its body, the copy of the function
/// it's in, the function's name if it's a function's, and the slot of
/// each of its locals that has one.
struct Frame<'p> {
    body: &'p crate::mir::Body,
    f: Option<&'p str>,
    slots: Vec<Option<Ptr>>,
}

/// Where a function or closure being run is: the `unsafe` blocks of its
/// body, the statement it's at, and the function's name, if it's one.
#[derive(Clone)]
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
            Value::Thread(id) => write!(f, "<thread {id}>"),
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
    let mut interp = Interp::new(program, term, 64, None);
    let result = interp.entry(program, entry);
    Ended {
        result,
        panicked: interp.panicked,
    }
}

/// Why working out a const's value failed (`evaluate`).
pub(crate) enum Failed {
    /// It aborted: panicked, or ran out of stack, with this error.
    Aborted(Error),
    /// It took more than its budget of steps.
    Budget,
}

/// The value const `entry` gives, worked out on a machine whose `usize` is
/// `bits` bits, in at most `budget` steps, each a basic block: as plain
/// data, which a const's type says it is (lemma C4 of docs/36-consts.md).
pub(crate) fn evaluate(
    program: &crate::mono::Executable,
    entry: &str,
    bits: u32,
    budget: u64,
) -> Result<crate::consts::Value, Failed> {
    let mut term = Captured::default();
    let mut interp = Interp::new(program, &mut term, bits, Some(budget));
    match interp.call(
        Value::Fn(entry.to_string(), None),
        Vec::new(),
        Span::new(0, 0),
    ) {
        Ok(v) => Ok(interp.tree(&v)),
        Err(_) if interp.out_of_steps => Err(Failed::Budget),
        Err(e) => Err(Failed::Aborted(e)),
    }
}

impl<'p, 'o> Interp<'p, 'o> {
    /// An interpreter of `program`, with `term` as its streams, on a
    /// machine whose `usize` is `bits` bits, that stops after `steps`
    /// basic blocks, if there's a limit.
    fn new(
        program: &'p crate::mono::Executable,
        term: &'o mut dyn Terminal,
        bits: u32,
        steps: Option<u64>,
    ) -> Interp<'p, 'o> {
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
        Interp {
            fns,
            structs,
            enums,
            nowhere,
            literals: HashMap::new(),
            statics: HashMap::new(),
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
            threads: threads::Threads::default(),
            exe: program,
            term,
            bits,
            steps,
            out_of_steps: false,
        }
    }

    /// Value `v`, of a const, as plain data: what a reference points to
    /// is bytes, of a literal, which are all a const's references can
    /// point to.
    fn tree(&self, v: &Value) -> crate::consts::Value {
        use crate::consts::Value as C;
        match v {
            Value::Int(n, _) => C::Int(*n),
            Value::Bool(b) => C::Bool(*b),
            Value::Char(c) => C::Char(*c),
            Value::Unit => C::Unit,
            Value::Struct(_, fields) => {
                C::Struct(fields.iter().map(|(_, f)| self.tree(f)).collect())
            }
            Value::Variant(ty, variant, fields) => {
                let i = (self.enums[ty.as_str()].variants.iter())
                    .position(|v| v.name.name == *variant)
                    .unwrap_or_else(|| unsound(&format!("no variant `{variant}` of `{ty}`")));
                C::Variant(i, fields.iter().map(|f| self.tree(f)).collect())
            }
            Value::Slots(values) => C::Array(values.iter().map(|f| self.tree(f)).collect()),
            Value::Ref(p) => {
                let whole = Ptr {
                    range: None,
                    ..p.clone()
                };
                let Value::Slots(slots) = self.memory.get(&whole) else {
                    unsound("a const's reference to what isn't bytes")
                };
                let (lo, len) = p.range.unwrap_or((0, slots.len()));
                let bytes = slots[lo..lo + len].iter().map(|b| match b {
                    Value::Int(n, IntTy::U8) => *n as u8,
                    other => unsound(&format!("a const's reference to `{other}`, not a byte")),
                });
                C::Bytes(bytes.collect())
            }
            other => unsound(&format!("a const's value `{other}` isn't plain data")),
        }
    }
}

impl<'p> Interp<'p, '_> {
    /// The least and greatest `t` on the machine this is (see `bits`).
    fn range(&self, t: IntTy) -> (i128, i128) {
        t.range_at(self.bits)
    }

    /// `n`, as a `t`, which it must be one of.
    fn int_of(&self, n: i128, t: IntTy) -> Value {
        let (lo, hi) = self.range(t);
        if !(lo..=hi).contains(&n) {
            unsound(&format!("{n} as {}", t.a()));
        }
        Value::Int(n, t)
    }

    /// The `t` that `n` is congruent to modulo 2^w, if `t` is `w` bits wide:
    /// `n` is right in its last `w` bits, which is all that's kept of a
    /// product that overflowed `i128`, as `wrapping_mul` is.
    fn wrap(&self, n: i128, t: IntTy) -> i128 {
        let (lo, hi) = self.range(t);
        let modulus = hi - lo + 1;
        let m = n.rem_euclid(modulus);
        if m > hi { m - modulus } else { m }
    }

    /// A `usize`, from a count the interpreter keeps.
    fn usize_of(&self, n: usize) -> Value {
        self.int_of(n as i128, IntTy::Usize)
    }

    /// `run_entry`'s work.
    fn entry(&mut self, program: &crate::mono::Executable, entry: &str) -> Result<Value, Error> {
        // `main` takes nothing, or the one `std::Os` there is, whose
        // arguments and environment are UTF-8.
        let params = self.fns[entry].params.len();
        if params > 0
            && let Some(what) = self.term.not_unicode()
        {
            return Err(Error::runtime(self.fns[entry].name.span, what.to_string()));
        }
        let args = match params {
            0 => Vec::new(),
            _ => vec![Value::Struct(
                "std::Os".into(),
                [
                    "stdin", "stdout", "stderr", "args", "env", "fs", "clock", "entropy", "threads",
                ]
                .into_iter()
                .zip([
                    "std::io::Stdin",
                    "std::io::Stdout",
                    "std::io::Stderr",
                    "std::env::Args",
                    "std::env::Env",
                    "std::fs::Fs",
                    "std::time::Clock",
                    "std::entropy::Entropy",
                    "std::thread::Threads",
                ])
                .map(|(field, ty)| (field.to_string(), Value::Struct(ty.into(), Vec::new())))
                .collect(),
            )],
        };
        // The interpreter's own panic, a bug's, waits for the threads to
        // give up before it goes on (`wind_down`), as they run on it.
        let main = Value::Fn(entry.to_string(), None);
        let call = std::panic::AssertUnwindSafe(|| self.call(main, args, Span::new(0, 0)));
        let result = match std::panic::catch_unwind(call) {
            Ok(result) => result,
            Err(p) => Err(self.stopped(p)),
        };
        let result = self.wind_down(result)?;
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
        // Each `ReadFile` is dropped, and its `discard` closes it, as a box
        // is freed: one left open is a file whose value was lost.
        if self.term.files_open() > 0 {
            unsound(&format!(
                "file leak: {} files were never closed",
                self.term.files_open()
            ));
        }
        // A `Raw` is dropped, and its `discard` puts the terminal back, by
        // the time `main` returns, as it holds the input `main` was given.
        if self.term.raw_on() {
            unsound("the terminal was left in raw mode");
        }
        Ok(result)
    }
}

/// Checks what free theorems say of a call of `f` with arguments `given`
/// that gave `result` (lemma P1 of docs/55-free-theorems.md): each part of
/// the result a free type parameter's way gets to, if it's there, is one of
/// the parts of the arguments its ways get to that are there. The checker
/// counts on it; nothing in T should be able to break it.
fn free_theorems(free: &[crate::ast::Free], given: &[Value], result: &Value, f: &str) {
    for p in free {
        let candidates: Vec<&Value> = (p.params.iter())
            .filter_map(|(i, way)| part(given.get(*i)?, way))
            .collect();
        for way in &p.result {
            if let Some(v) = part(result, way)
                && !candidates.contains(&v)
            {
                unsound(&format!(
                    "what `{f}` gave of its free type parameter `{}`, `{v}`, isn't one of \
                     its arguments' values of it",
                    p.name
                ));
            }
        }
    }
}

/// The part of `v` that `way` gets to, if it's there: each enum on the way
/// is the variant the way goes through.
fn part<'v>(v: &'v Value, way: &[(String, usize)]) -> Option<&'v Value> {
    let mut v = v;
    for (variant, field) in way {
        match v {
            Value::Variant(_, name, fields) if name == variant => v = fields.get(*field)?,
            Value::Variant(..) => return None,
            _ => unsound(&format!(
                "a free theorem's way into `{v}`, which isn't an enum's value"
            )),
        }
    }
    Some(v)
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
    /// How many bits a `usize` is on the machine this is, 32 or 64.
    bits: u32,
    /// How many more basic blocks it may run, if there's a limit: there is
    /// for a const, worked out while compiling (`evaluate`).
    steps: Option<u64>,
    /// Did it stop because `steps` ran out?
    out_of_steps: bool,
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
    /// Where each static is, by symbol, once it's been used: a slot made
    /// then from its value, worked out while compiling, so it doesn't
    /// matter when (lemma Y6 of docs/38-statics.md). It's never freed.
    statics: HashMap<String, Ptr>,
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
    /// The program's threads, and which is running (see `threads`).
    threads: threads::Threads<'p>,
    /// The operating system's streams, as the program's `std::Os` reaches
    /// them.
    term: &'o mut dyn Terminal,
}

/// The streams of the operating system, for the intrinsics of `std::io`'s
/// `Stdin`, `Stdout` and `Stderr` (see docs/19-std-io.md and
/// docs/47-io.md), and what the program was started with, for `std::env`'s:
/// the process's own, for `t run`, or in memory, for tests.
pub trait Terminal {
    /// Reads into `buf`: how many bytes, at least one unless the input
    /// has ended or `buf` is empty, without waiting for more.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Writes all of `buf` to the output, or to where errors go if `err`.
    fn write(&mut self, err: bool, buf: &[u8]) -> std::io::Result<()>;

    /// Writes out what waits in the output's buffer, or that of where
    /// errors go if `err`.
    fn flush(&mut self, err: bool) -> std::io::Result<()>;

    /// Is the stream a terminal: the input, if `stream` is `None`, or else
    /// the output, or where errors go if it's `Some(true)`?
    fn is_terminal(&self, stream: Option<bool>) -> bool;

    /// What the program was started as, then its arguments, each UTF-8.
    fn args(&self) -> &[String];

    /// The variables of its environment, each name with its value.
    fn env(&self) -> &[(String, String)];

    /// What isn't UTF-8 of what the program was started with, if
    /// something isn't: then a program given the `Os` doesn't start
    /// (docs/47-io.md, lemma A1).
    fn not_unicode(&self) -> Option<&str> {
        None
    }

    /// Opens the file at `path`, which has no `\0`, to be read, for
    /// `std::fs::Fs::open`: its number, by which it's read and closed.
    fn open(&mut self, path: &str) -> std::io::Result<usize>;

    /// Reads into `buf` from open file `file`, as `read` does from the
    /// input: what's buffered, or else what one read gives.
    fn read_file(&mut self, file: usize, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Closes open file `file`, which isn't read again.
    fn close(&mut self, file: usize);

    /// Writes all of `bytes` to the file at `path`, made if there's none,
    /// after what it holds if `append`, or else instead of it.
    fn write_file(&mut self, path: &str, bytes: &[u8], append: bool) -> std::io::Result<()>;

    /// Removes the file at `path`.
    fn remove(&mut self, path: &str) -> std::io::Result<()>;

    /// How many files are open.
    fn files_open(&self) -> usize;

    /// Now, by a clock that only goes forward, in nanoseconds since some
    /// moment before the program began: for `std::time::Clock::now`.
    fn now(&self) -> u64 {
        monotonic_now()
    }

    /// Waits until `now()` is no less than `deadline`.
    fn sleep_until(&mut self, deadline: u64) {
        loop {
            let now = self.now();
            if now >= deadline {
                return;
            }
            std::thread::sleep(std::time::Duration::from_nanos(deadline - now));
        }
    }

    /// The nanoseconds since 1970 by the time of day, or `None` if that's
    /// before 1970.
    fn unix_time(&self) -> Option<u64> {
        let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        since
            .ok()
            .map(|d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX - 1))
    }

    /// Fills `buf` with entropy, as the operating system's.
    fn fill_entropy(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        os_entropy(buf)
    }

    /// Puts the terminal the input is in raw mode, keeping how it was:
    /// `ENOTTY` if the input isn't one.
    fn raw_enter(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(ENOTTY))
    }

    /// Puts the terminal back as `raw_enter` found it.
    fn raw_leave(&mut self) {
        unsound("raw mode left, but never entered")
    }

    /// Is the terminal in raw mode?
    fn raw_on(&self) -> bool {
        false
    }

    /// Waits until there's input to read, or `now()` is no less than
    /// `deadline`: whether there's input. Only in raw mode.
    fn wait(&mut self, _deadline: u64) -> std::io::Result<bool> {
        unsound("a wait for input, not in raw mode")
    }

    /// The size of the terminal the output goes to, or where errors go if
    /// `err`: its columns and rows, or `ENOTTY` if it isn't one.
    fn size(&self, _err: bool) -> std::io::Result<(u16, u16)> {
        Err(std::io::Error::from_raw_os_error(ENOTTY))
    }
}

/// The failure of an operating system's asked about a terminal of
/// something that isn't one: `ENOTTY`, which is 25 wherever T runs.
pub const ENOTTY: i32 = 25;

/// Now, by `std::time::Instant`, in nanoseconds since the first time this
/// was asked: as a compiled program's `CLOCK_MONOTONIC`, from some moment
/// before the program began, one past it so even the first is after it.
pub fn monotonic_now() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = *START.get_or_init(std::time::Instant::now);
    u64::try_from(start.elapsed().as_nanos())
        .unwrap_or(u64::MAX)
        .saturating_add(1)
}

/// Fills `buf` with the operating system's entropy, as a compiled
/// program's `getentropy` does: the operating system's own failure, or
/// `Unsupported` where it has none to give.
pub fn os_entropy(buf: &mut [u8]) -> std::io::Result<()> {
    getrandom::fill(buf).map_err(|e| match e.raw_os_error() {
        Some(code) => std::io::Error::from_raw_os_error(code),
        None => std::io::Error::from(std::io::ErrorKind::Unsupported),
    })
}

/// The failure of an operating system's that has no file at the path it
/// was given: `ENOENT`, which is 2 wherever T runs.
fn not_found() -> std::io::Error {
    std::io::Error::from_raw_os_error(2)
}

/// Streams in memory: input given ahead, and what's written kept, none a
/// terminal; and the arguments and environment the program is given.
#[derive(Default)]
pub struct Captured {
    pub input: std::collections::VecDeque<u8>,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
    /// What the program was started as, then its arguments. Empty is as
    /// `t-program` and none.
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// The files, in memory, by path: one directory, with no others in it,
    /// so a path with a `/` names no file.
    pub files: std::collections::BTreeMap<String, Vec<u8>>,
    /// The files open to be read, by number: what each held when it was
    /// opened, and how much of that has been read. `None` once closed.
    pub open: Vec<Option<(Vec<u8>, usize)>>,
}

impl Captured {
    /// Reads `input`, with the arguments `args` after the program's name
    /// `program`, and the environment `env`.
    pub fn new(
        input: Vec<u8>,
        program: &str,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Captured {
        Captured {
            input: input.into(),
            args: std::iter::once(program.to_string()).chain(args).collect(),
            env,
            ..Captured::default()
        }
    }
}

impl Terminal for Captured {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(self.input.len());
        for (b, i) in buf.iter_mut().zip(self.input.drain(..n)) {
            *b = i;
        }
        Ok(n)
    }

    fn write(&mut self, err: bool, buf: &[u8]) -> std::io::Result<()> {
        match err {
            true => self.err.extend_from_slice(buf),
            false => self.out.extend_from_slice(buf),
        }
        Ok(())
    }

    fn flush(&mut self, _err: bool) -> std::io::Result<()> {
        Ok(())
    }

    fn is_terminal(&self, _stream: Option<bool>) -> bool {
        false
    }

    fn args(&self) -> &[String] {
        static NONE: std::sync::LazyLock<[String; 1]> =
            std::sync::LazyLock::new(|| ["t-program".to_string()]);
        match self.args.is_empty() {
            true => &*NONE,
            false => &self.args,
        }
    }

    fn env(&self) -> &[(String, String)] {
        &self.env
    }

    fn open(&mut self, path: &str) -> std::io::Result<usize> {
        let held = self.files.get(path).ok_or_else(not_found)?.clone();
        self.open.push(Some((held, 0)));
        Ok(self.open.len() - 1)
    }

    fn read_file(&mut self, file: usize, buf: &mut [u8]) -> std::io::Result<usize> {
        let Some(Some((held, at))) = self.open.get_mut(file) else {
            unsound("a read of a file that isn't open")
        };
        let n = buf.len().min(held.len() - *at);
        buf[..n].copy_from_slice(&held[*at..*at + n]);
        *at += n;
        Ok(n)
    }

    fn close(&mut self, file: usize) {
        match self.open.get_mut(file) {
            Some(f @ Some(_)) => *f = None,
            _ => unsound("a file closed twice, or never opened"),
        }
    }

    fn write_file(&mut self, path: &str, bytes: &[u8], append: bool) -> std::io::Result<()> {
        if path.is_empty() || path.contains('/') {
            return Err(not_found());
        }
        let held = self.files.entry(path.to_string()).or_default();
        if !append {
            held.clear();
        }
        held.extend_from_slice(bytes);
        Ok(())
    }

    fn remove(&mut self, path: &str) -> std::io::Result<()> {
        self.files.remove(path).map(|_| ()).ok_or_else(not_found)
    }

    fn files_open(&self) -> usize {
        self.open.iter().filter(|f| f.is_some()).count()
    }
}

/// What a stream's intrinsic gives back for `e`: bit 63, the `ErrorKind`
/// by its place in `core::io`'s enum in bits 32 to 39, and the operating
/// system's number for it, if it has one, in the low 32 bits, as
/// `std::io::failure` reads it.
pub fn io_failure(e: &std::io::Error) -> u64 {
    use std::io::ErrorKind as K;
    let kind: u64 = match e.kind() {
        K::NotFound => 0,
        K::PermissionDenied => 1,
        K::AlreadyExists => 2,
        K::BrokenPipe => 3,
        K::InvalidData => 4,
        K::UnexpectedEof => 5,
        K::Unsupported => 6,
        _ if e.raw_os_error() == Some(ENOTTY) => 6,
        _ => 8,
    };
    let code = e.raw_os_error().unwrap_or(0) as u32;
    (1 << 63) | (kind << 32) | u64::from(code)
}

/// What an intrinsic that gives back 0 or a failure gives back for `r`.
fn status(r: std::io::Result<()>) -> Value {
    let r = match r {
        Ok(()) => 0,
        Err(e) => io_failure(&e),
    };
    Value::Int(i128::from(r), IntTy::U64)
}

/// A refinement to check at run time (see `check/refine.rs`): its binder,
/// predicate, and the values of the other names in it, which can't change.
#[derive(Clone)]
struct Refined {
    /// Each binder and predicate, with the ways to the parts of the value
    /// it's of: the whole value's, and those of its enums' type arguments
    /// (`Measures::ways`).
    parts: Vec<RefinedPart>,
    env: Vec<(String, Value)>,
    /// What each measure the predicate calls of a name other than its
    /// binder gives, by node: found when it's made, as the predicate is
    /// about then, and what's behind a reference may change after.
    measured: HashMap<NodeId, PredValue>,
}

/// One predicate of a `Refined`.
#[derive(Clone)]
struct RefinedPart {
    binder: String,
    pred: Expr,
    /// Through which variant and field of each enum on the way to each
    /// part of the value it's of: one empty way for the whole value.
    ways: Vec<Vec<(String, usize)>>,
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
    /// which is `what`: each part of it, of each part of `v` it's of that
    /// `v` has, being those variants.
    fn check(&self, i: &Interp, v: &Value, what: &str) {
        for part in &self.parts {
            for way in &part.ways {
                let mut at = v;
                let mut there = true;
                for (variant, field) in way {
                    match at {
                        Value::Variant(_, name, fields) if name == variant => at = &fields[*field],
                        _ => {
                            there = false;
                            break;
                        }
                    }
                }
                if !there {
                    continue;
                }
                if let PredValue::Bool(false) = self.eval(i, &part.pred, &part.binder, at) {
                    let what = match way.is_empty() {
                        true => what.to_string(),
                        false => format!("{what}, of which a part"),
                    };
                    unsound(&format!(
                        "{what} is `{at}`, which doesn't have its refinement `{}`",
                        crate::pretty::expr_text(&part.pred)
                    ));
                }
            }
        }
    }

    fn eval(&self, i: &Interp, e: &Expr, binder: &str, v: &Value) -> PredValue {
        let go = |e: &Expr| self.eval(i, e, binder, v);
        match &e.kind {
            ExprKind::Int(n, _) => PredValue::Int(i128::from(*n)),
            ExprKind::Bool(b) => PredValue::Bool(*b),
            ExprKind::Var(name, _) if name == binder => PredValue::of(v),
            ExprKind::Var(name, _) => match self.env.iter().find(|(n, _)| n == name) {
                Some((_, v)) => PredValue::of(v),
                None => unsound(&format!("a refinement naming `{name}`, which isn't there")),
            },
            ExprKind::Unary(UnOp::Neg, x) => PredValue::Int(-go(x).int()),
            ExprKind::Unary(UnOp::Not, x) => PredValue::Bool(!go(x).boolean()),
            ExprKind::Binary(op, l, r) => pred_op(*op, go(l), go(r)),
            ExprKind::MethodCall(x, ..) => match self.measured.get(&e.id) {
                Some(m) => m.clone(),
                None => i.measure_call(e.id, go(x).val()),
            },
            ExprKind::Field(x, name) => match self.measured.get(&e.id) {
                Some(m) => m.clone(),
                None => i.pred_field(go(x).val(), &name.name),
            },
            // `old(x)` in a postcondition: its measures were found at the
            // call.
            ExprKind::Call(_, args) if args.len() == 1 => go(&args[0]),
            _ => unsound("a refinement outside the logic"),
        }
    }
}

/// Each measure call and field in predicate `e`, with the name its path
/// is from (`s` of `s.head.len()`): in a postcondition (`post`), only those
/// inside `old(..)` and those not of what's behind a `&mut` (`at_return`
/// has those), which are the same at the call as at return.
fn each_measure_call<'e>(
    e: &'e Expr,
    post: bool,
    at_return: &HashSet<NodeId>,
    out: &mut Vec<(&'e Expr, &'e str)>,
) {
    match &e.kind {
        ExprKind::MethodCall(..) | ExprKind::Field(..) if !post || !at_return.contains(&e.id) => {
            if let Some(root) = path_root(e) {
                out.push((e, root));
            }
        }
        ExprKind::Call(_, args) if args.len() == 1 => {
            return each_measure_call(&args[0], false, at_return, out);
        }
        _ => {}
    }
    for c in e.children() {
        each_measure_call(c, post, at_return, out);
    }
}

/// The name path `e` of a predicate is from: `s` of `s.head.len()`.
fn path_root(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Var(name, _) => Some(name),
        ExprKind::Field(x, _) | ExprKind::MethodCall(x, ..) => path_root(x),
        _ => None,
    }
}

impl<'p> Interp<'p, '_> {
    /// The refinements written in type `t`, if it has any, whose other
    /// names have the values `env`: what the measures they call of them
    /// give is found now, or of a postcondition (`post`), as `refined_at`.
    fn type_refined(&self, t: &TypeExpr, env: &[(String, Value)], post: bool) -> Option<Refined> {
        let parts: Vec<(String, &Expr)> = (t.refinements().into_iter())
            .map(|(_, v, pred)| (v.name.clone(), pred))
            .collect();
        (!parts.is_empty()).then(|| self.refined_at(&parts, env.to_vec(), post))
    }

    /// The refinement with parts `parts`, each a binder and a predicate,
    /// as `refined`; of a postcondition (`post`): what's behind its names
    /// is found when it's checked, at return, but for what's in `old(..)`.
    fn refined_at(
        &self,
        parts: &[(String, &Expr)],
        env: Vec<(String, Value)>,
        post: bool,
    ) -> Refined {
        let mut r = Refined {
            parts: (parts.iter())
                .map(|(binder, pred)| RefinedPart {
                    binder: binder.clone(),
                    pred: (*pred).clone(),
                    ways: match self.measures.ways.get(&pred.id) {
                        Some(ways) => ways.clone(),
                        None => vec![Vec::new()],
                    },
                })
                .collect(),
            env,
            measured: HashMap::new(),
        };
        for (binder, pred) in parts {
            let mut calls = Vec::new();
            each_measure_call(pred, post, &self.measures.behind_mut, &mut calls);
            for (e, root) in calls {
                if root != binder {
                    let m = r.eval(self, e, binder, &Value::Unit);
                    r.measured.insert(e.id, m);
                }
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
                // How many there are of what the program was started with.
                ("<Args>::len", _) => PredValue::Int(self.term.args().len() as i128 - 1),
                ("<Env>::len", _) => PredValue::Int(self.term.env().len() as i128),
                (_, this) => unsound(&format!("intrinsic measure `{symbol}` of `{this}`")),
            },
            None => unsound(&format!("`{symbol}` isn't a measure")),
        }
    }

    /// Field `name` of `v`, or of what it points to, through references
    /// and boxes: of a predicate's path, or a measure's body.
    fn pred_field(&self, mut v: Value, name: &str) -> PredValue {
        while let Value::Ref(p) | Value::Box(p) = &v
            && p.range.is_none()
        {
            v = self.pred_deref(v);
        }
        match &v {
            Value::Struct(_, fields) => match fields.iter().find(|(n, _)| *n == name) {
                Some((_, f)) => PredValue::of(f),
                None => unsound(&format!("no field `{name}` in `{v}`")),
            },
            _ => unsound(&format!("field `{name}` of `{v}`")),
        }
    }

    /// The value of term `e` of a measure's body, whose `self` is `this`.
    fn measure_term(&self, e: &Expr, this: &Value) -> PredValue {
        match &e.kind {
            ExprKind::Int(n, _) => PredValue::Int(i128::from(*n)),
            ExprKind::Bool(b) => PredValue::Bool(*b),
            ExprKind::Var(..) => PredValue::Val(this.clone()),
            ExprKind::Field(x, name) => {
                self.pred_field(self.measure_term(x, this).val(), &name.name)
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
    /// wrote, or for an arena's `Slots`, by freeing its values, first put
    /// in first, each by the glue of its box.
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
        match exe.body(&name) {
            Some(body) => self.run(body, None, vec![v], Vec::new(), span).map(|_| ()),
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
            // A const's initializer may panic, while compiling, where it's
            // run: once it's known not to, it's its value (`crate::consts`).
            // A static's too, which is run to make the static, at its
            // first use.
            if self.fns[f.as_str()].is_const() {
                continue;
            }
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
                let body = (exe.body(&name))
                    .unwrap_or_else(|| unsound(&format!("glue `{name}` has no body")));
                self.run(body, None, args, Vec::new(), span)
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
                let refinement = |i: &Self, t: &TypeExpr| i.type_refined(t, &env, false);
                // A result's refinement and `ensures` are about what's
                // behind the arguments at return.
                let ret = self.type_refined(&decl.ret, &env, true);
                let ensures: Vec<Refined> = (decl.ensures.iter())
                    .map(|e| self.refined_at(&[(String::new(), e)], env.clone(), true))
                    .collect();
                // The arguments as they're given, if a free theorem says
                // what the result is of them.
                let free = self.measures.free.get(&decl.name.id);
                let given = free.map(|_| args.clone());
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
                // What a C function does is the machine's: there's none here
                // (docs/40-extern.md, I).
                let result = if decl.is_import() {
                    let message = format!(
                        "`{}` is a C function, which the interpreter can't call: build the \
                         program, with `t build`, to run it",
                        decl.name.name
                    );
                    Err(self.panic(span, message))
                } else if decl.intrinsic.is_some() {
                    let which = (self.exe.intrinsic(&name))
                        .unwrap_or_else(|| unsound(&format!("`{name}` is no intrinsic")));
                    self.intrinsic(which, &name, args, span)
                } else {
                    let exe = self.exe;
                    let (name, body) = (exe.body_named(&name))
                        .unwrap_or_else(|| unsound(&format!("`{name}` has no body")));
                    self.frames.push(name.to_string());
                    let result = self.run(body, Some(name), args, refined, span);
                    self.frames.pop();
                    result
                };
                if let (Ok(v), Some(r)) = (&result, ret) {
                    r.check(self, v, &format!("the result of `{}`", decl.name.name));
                }
                if let (Ok(v), Some(free), Some(given)) = (&result, free, &given) {
                    free_theorems(free, given, v, &decl.name.name);
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
                let body = (exe.body(&c.code))
                    .unwrap_or_else(|| unsound(&format!("closure `{}` has no body", c.code)));
                match env_by(body) {
                    None => {
                        let mut all = vec![Value::Closure(c.clone())];
                        all.extend(args);
                        self.run(body, None, all, Vec::new(), span)
                    }
                    Some(_) => {
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
    /// as it's `Fn` or `FnMut`.
    fn call_closure(&mut self, at: &Ptr, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        let Value::Closure(c) = self.memory.get(at).clone() else {
            unreachable!("a closure")
        };
        let exe = self.exe;
        let body = (exe.body(&c.code))
            .unwrap_or_else(|| unsound(&format!("closure `{}` has no body", c.code)));
        if env_by(body).is_none() {
            unsound("a closure that's `FnOnce` called through a reference");
        }
        let all = std::iter::once(Value::Ref(at.clone()))
            .chain(args)
            .collect();
        self.run(body, None, all, Vec::new(), span)
    }

    /// Runs MIR `body`, of function `f` or else of a closure or glue, with
    /// its parameters `args`, the first the closure if it's a closure's, those with
    /// refinements having them, called at `span`. Each local's slot is made
    /// when it's first given a value, and freed when it goes out of scope
    /// (`StmtKind::Dead`), which drops what's in it.
    fn run(
        &mut self,
        body: &'p crate::mir::Body,
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
        let result = self.run_body(body, f, args, refined, span);
        self.calls.pop();
        result
    }

    /// `run`, once the call is on `calls`.
    fn run_body(
        &mut self,
        body: &'p crate::mir::Body,
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
                    StmtKind::Holds(r) => {
                        self.holds(&fr, &body.refinements[*r]);
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
            if let Some(n) = &mut self.steps {
                if *n == 0 {
                    self.out_of_steps = true;
                    let at = self.sites.last().expect("pushed").span;
                    break 'run Err(self.panic(at, "out of steps"));
                }
                *n -= 1;
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
                Const::Int(n, crate::types::Type::Int(t)) => self.int_of(*n, *t),
                Const::Int(_, t) => unsound(&format!("an integer of type `{t}`")),
                Const::Bool(b) => Value::Bool(*b),
                Const::Char(c) => Value::Char(*c),
                Const::Unit => Value::Unit,
                Const::Str(s) => Value::Ref(self.literal(s.as_bytes())),
                Const::Bytes(b) => Value::Ref(self.literal(b)),
                Const::Fn(name, _) => Value::Fn(name.clone(), None),
                Const::Param(_) => unsound("a const parameter left after monomorphising"),
                Const::Static(symbol) => Value::Ref(self.static_place(symbol)),
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
                            let (lo, hi) = self.range(t);
                            self.int_of(lo + hi - n, t)
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
                        let (lo, hi) = self.range(*t);
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
                    // Its body is its type's, wherever it's made: in the
                    // function the closure is in, or in its `{fn}` glue.
                    (Agg::Closure(_), t) => {
                        let exe = self.exe;
                        let code = (exe.closure_body(t))
                            .unwrap_or_else(|| unsound(&format!("closure type `{t}` has no body")))
                            .to_string();
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
        let parts: Vec<(String, &Expr)> = (r.parts.iter())
            .map(|(binder, pred)| (binder.clone(), pred))
            .collect();
        let refined = self.refined_at(&parts, env, false);
        let p = self.ptr(fr, &crate::mir::Place::local(l));
        let name = fr.body.locals[l.0].name.as_deref().unwrap_or("_");
        refined.check(self, self.memory.get(&p), &format!("`{name}`"));
        self.refined.insert((p.slot, p.generation), refined);
    }

    /// Stops the program, as the checker is wrong, unless loop invariant
    /// `r` holds of the variables it names, now, at the loop's head.
    fn holds(&mut self, fr: &Frame, r: &crate::mir::Refinement) {
        let env = (r.env.iter())
            .map(|(n, place)| {
                let p = self.ptr(fr, place);
                (n.clone(), self.memory.get(&p).clone())
            })
            .collect();
        let parts: Vec<(String, &Expr)> = (r.parts.iter())
            .map(|(binder, pred)| (binder.clone(), pred))
            .collect();
        let refined = self.refined_at(&parts, env, false);
        for part in &refined.parts {
            if let PredValue::Bool(false) =
                refined.eval(self, &part.pred, &part.binder, &Value::Unit)
            {
                unsound(&format!(
                    "a loop's invariant `{}` doesn't hold at its head",
                    crate::pretty::expr_text(&part.pred)
                ));
            }
        }
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
            (Pat::Str(s), v @ Value::Ref(..)) => self.bytes(v) == s.as_bytes(),
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
    /// Runs intrinsic `which`, whose copy is `copy`, on `args`, called at
    /// `span`.
    fn intrinsic(
        &mut self,
        which: Intrinsic,
        copy: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Error> {
        match which {
            // A device's memory is the machine's: there's none here
            // (docs/40-extern.md, I). Without an address, there's nothing to
            // read or write.
            Intrinsic::Volatile(VolatileOp::At) => {
                let message = "`Volatile::at` names an address of the machine's, which the \
                               interpreter hasn't got: build the program, with `t build`, to \
                               run it";
                Err(self.panic(span, message))
            }
            Intrinsic::Volatile(VolatileOp::Read | VolatileOp::Write) => {
                unsound("a volatile value the interpreter never made")
            }
            Intrinsic::WrappingMul => {
                let [a, b] =
                    <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("checked arity"));
                let (Value::Int(a, t), Value::Int(b, u)) = (a, b) else {
                    unsound("`wrapping_mul` of what isn't an integer")
                };
                if t != u {
                    unsound("`wrapping_mul` of two integer types");
                }
                Ok(self.int_of(self.wrap(a.wrapping_mul(b), t), t))
            }
            Intrinsic::Wrap => {
                let [Value::Int(x, _)] =
                    <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("checked arity"))
                else {
                    unsound("`wrap` of what isn't an integer")
                };
                let u = int_ty(&self.fns[copy].ret);
                Ok(self.int_of(self.wrap(x, u), u))
            }
            Intrinsic::Least | Intrinsic::Greatest => {
                let t = int_ty(&self.fns[copy].ret);
                let (lo, hi) = self.range(t);
                Ok(Value::Int(
                    if which == Intrinsic::Least { lo } else { hi },
                    t,
                ))
            }
            Intrinsic::Message(op) => self.message(op, args, span),
            Intrinsic::Stdin(op) => Ok(self.stdin(op, args)),
            Intrinsic::Output(out, op) => Ok(self.output(out, op, args)),
            Intrinsic::Args(op) => Ok(self.args(op, args)),
            Intrinsic::Env(op) => Ok(self.env(op, copy, args)),
            Intrinsic::Fs(op) => Ok(self.fs(op, copy, args)),
            Intrinsic::ReadFile(op) => Ok(self.read_file(op, args)),
            Intrinsic::Clock(op) => Ok(self.clock(op, args)),
            Intrinsic::EntropyFill => Ok(self.entropy(args)),
            Intrinsic::UnsafeAsMut | Intrinsic::UnsafeAsRef => Ok(self.unsafe_cell(args)),
            Intrinsic::Atomic(op) => {
                // What another thread does with it can be seen here, and what
                // this one does there.
                self.turn()?;
                Ok(self.atomic(op, args))
            }
            Intrinsic::Spin => {
                self.turn()?;
                Ok(Value::Unit)
            }
            Intrinsic::ThreadStart => self.start(args, span),
            Intrinsic::ThreadJoin => self.join(args, span),
            Intrinsic::Slots(op) => Ok(self.arena(op, args)),
            Intrinsic::RawBuf(op) => self.raw_buf(op, args, span),
            // A string is its bytes.
            Intrinsic::StrAsBytes => {
                let [s] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
                Ok(s)
            }
            Intrinsic::StrLen => Ok(self.slice(SliceOp::Len, copy, args)),
            Intrinsic::Slice(op) => Ok(self.slice(op, copy, args)),
            Intrinsic::Array(op) => Ok(self.array(op, args)),
            // No slots: of any type, as there's nothing there.
            Intrinsic::EmptySlice { .. } => Ok(Value::Ref(self.nowhere.slice(0, 0))),
            Intrinsic::OutOfMemory => Err(self.out_of_memory(span, "out of memory")),
            // `f(0)`, `f(1)`, .., each made in place, as many as the copy's
            // array type says, calling `f` where it is. On a panic, what's
            // made is forgotten: the program is aborting.
            Intrinsic::ArrayFromFn => {
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
                    values.push(self.call(f.clone(), vec![self.usize_of(i)], span)?);
                }
                Ok(Value::Slots(values))
            }
            // Aborts, saying why.
            Intrinsic::Panic => {
                let [why] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("panic arity"));
                let why = self.text(&why);
                Err(self.panics(span, format!("panicked: {why}")))
            }
            // The same bytes: a `str`'s are UTF-8, which is what it adds.
            Intrinsic::FromUtf8Unchecked => {
                let [b] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
                let block = self.unsafe_op();
                if std::str::from_utf8(&self.bytes(&b)).is_err() {
                    let why = "`str::from_utf8_unchecked` of bytes that aren't UTF-8".to_string();
                    return Err(self.broken(span, block, why));
                }
                Ok(b)
            }
            // Exchanges two values, making and throwing away none.
            Intrinsic::Swap => {
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
        }
    }

    /// The intrinsic method `op` of `core::panic::Message`, the message
    /// `panic_with` writes, with arguments `args`. There's only ever one
    /// being written: `fmt` into it can't panic, so can't start another.
    fn message(&mut self, op: MessageOp, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        match (op, args.as_slice()) {
            (MessageOp::New, []) => {
                self.message.clear();
                Ok(Value::Struct("core::panic::Message".into(), Vec::new()))
            }
            (MessageOp::PushStr, [_, s]) => {
                let s = self.text(s);
                self.message.push_str(&s);
                Ok(Value::Unit)
            }
            (MessageOp::PushChar, [_, Value::Char(c)]) => {
                self.message.push(*c);
                Ok(Value::Unit)
            }
            (MessageOp::Abort, [_]) => {
                let why = std::mem::take(&mut self.message);
                Err(self.panics(span, format!("panicked: {why}")))
            }
            _ => unsound(&format!(
                "`Message::{}` of the wrong arguments",
                Intrinsic::Message(op).name()
            )),
        }
    }

    /// An intrinsic method of `std::io::Stdin`, with arguments `args`.
    fn stdin(&mut self, op: StdinOp, args: Vec<Value>) -> Value {
        match (op, args.as_slice()) {
            (StdinOp::Read, [_, buf]) => {
                let Value::Ref(
                    p @ Ptr {
                        range: Some((lo, len)),
                        ..
                    },
                ) = buf
                else {
                    unsound("`os_read` into something that isn't a slice")
                };
                let mut bytes = vec![0; *len];
                let r = loop {
                    match self.term.read(&mut bytes) {
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                        r => break r,
                    }
                };
                let n = match r {
                    Ok(n) => n.min(*len),
                    Err(e) => return Value::Int(i128::from(io_failure(&e)), IntTy::U64),
                };
                let (p, lo) = (p.clone(), *lo);
                let Value::Slots(slots) = self.memory.get_mut(&p) else {
                    unsound("a slice that isn't of slots")
                };
                for (i, &b) in bytes[..n].iter().enumerate() {
                    slots[lo + i] = Value::Int(i128::from(b), IntTy::U8);
                }
                Value::Int(n as i128, IntTy::U64)
            }
            (StdinOp::IsTerminal, [_]) => Value::Bool(self.term.is_terminal(None)),
            (StdinOp::RawEnter, [_]) => {
                // Only one `Raw` lives at a time, as it holds the one
                // `Stdin` by `&mut` (lemma T3 of docs/49-time-entropy-terminal.md).
                if self.term.raw_on() {
                    unsound("raw mode entered twice");
                }
                status(self.term.raw_enter())
            }
            (StdinOp::RawLeave, [_]) => {
                self.term.raw_leave();
                Value::Unit
            }
            (StdinOp::Wait, [_, Value::Int(deadline, _)]) => {
                match self.term.wait(*deadline as u64) {
                    Ok(ready) => Value::Int(i128::from(ready), IntTy::U64),
                    Err(e) => Value::Int(i128::from(io_failure(&e)), IntTy::U64),
                }
            }
            _ => unsound(&format!("`Stdin`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// An intrinsic method of `std::io::Stdout` or `Stderr`, `out`, with
    /// arguments `args`.
    fn output(&mut self, out: Output, op: OutputOp, args: Vec<Value>) -> Value {
        let err = out == Output::Stderr;
        let done = match (op, args.as_slice()) {
            (OutputOp::Write, [_, buf]) => {
                let bytes = self.bytes(buf);
                self.term.write(err, &bytes)
            }
            (OutputOp::Flush, [_]) => self.term.flush(err),
            (OutputOp::IsTerminal, [_]) => return Value::Bool(self.term.is_terminal(Some(err))),
            (OutputOp::Size, [_]) => {
                let r = match self.term.size(err) {
                    Ok((cols, rows)) => u64::from(cols) << 16 | u64::from(rows),
                    Err(e) => io_failure(&e),
                };
                return Value::Int(i128::from(r), IntTy::U64);
            }
            _ => unsound(&format!("`{out:?}`'s `{op:?}` of the wrong arguments")),
        };
        let r = match done {
            Ok(()) => 0,
            Err(e) => io_failure(&e),
        };
        Value::Int(i128::from(r), IntTy::U64)
    }

    /// An intrinsic method of `std::time::Clock`, with arguments `args`.
    fn clock(&mut self, op: ClockOp, args: Vec<Value>) -> Value {
        match (op, args.as_slice()) {
            (ClockOp::Now, [_]) => Value::Int(i128::from(self.term.now()), IntTy::U64),
            (ClockOp::SleepUntil, [_, Value::Int(deadline, _)]) => {
                self.term.sleep_until(*deadline as u64);
                Value::Unit
            }
            (ClockOp::UnixTime, [_]) => Value::Int(
                i128::from(self.term.unix_time().unwrap_or(u64::MAX)),
                IntTy::U64,
            ),
            _ => unsound(&format!("`Clock`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// `std::entropy::Entropy::os_fill`, with arguments `args`.
    fn entropy(&mut self, args: Vec<Value>) -> Value {
        let [_, buf] = args.as_slice() else {
            unsound("`os_fill` of the wrong arguments")
        };
        let Value::Ref(
            p @ Ptr {
                range: Some((lo, len)),
                ..
            },
        ) = buf
        else {
            unsound("`os_fill` into something that isn't a slice")
        };
        let (p, lo, len) = (p.clone(), *lo, *len);
        let mut bytes = vec![0; len];
        if let Err(e) = self.term.fill_entropy(&mut bytes) {
            return Value::Int(i128::from(io_failure(&e)), IntTy::U64);
        }
        let Value::Slots(slots) = self.memory.get_mut(&p) else {
            unsound("a slice that isn't of slots")
        };
        for (i, &b) in bytes.iter().enumerate() {
            slots[lo + i] = Value::Int(i128::from(b), IntTy::U8);
        }
        Value::Int(0, IntTy::U64)
    }

    /// An intrinsic method of `std::fs::Fs`, whose copy is `copy`, with
    /// arguments `args`. A `ReadFile` is a struct of one field the program
    /// can't name, its number in the terminal's open files.
    fn fs(&mut self, op: FsOp, copy: &str, args: Vec<Value>) -> Value {
        match (op, args.as_slice()) {
            (FsOp::Open, [_, path]) => {
                let path = self.text(path);
                let TypeExprKind::Named(result, _) = &self.fns[copy].ret.kind else {
                    unsound("`Fs::os_open` gives a `Result`")
                };
                let result = result.clone();
                match self.term.open(&path) {
                    Ok(n) => {
                        let file = Value::Struct(
                            "std::fs::ReadFile".into(),
                            vec![("file".into(), self.usize_of(n))],
                        );
                        Value::Variant(result, "Ok".into(), vec![file])
                    }
                    Err(e) => Value::Variant(
                        result,
                        "Err".into(),
                        vec![Value::Int(i128::from(io_failure(&e)), IntTy::U64)],
                    ),
                }
            }
            (FsOp::Write, [_, path, bytes, Value::Bool(append)]) => {
                let (path, bytes) = (self.text(path), self.bytes(bytes));
                status(self.term.write_file(&path, &bytes, *append))
            }
            (FsOp::Remove, [_, path]) => {
                let path = self.text(path);
                status(self.term.remove(&path))
            }
            _ => unsound(&format!("`Fs`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// An intrinsic method of `std::fs::ReadFile`, with arguments `args`.
    fn read_file(&mut self, op: ReadFileOp, args: Vec<Value>) -> Value {
        // The number of the file `v` is, or a reference to one.
        let number = |interp: &Self, v: &Value| -> usize {
            let v = match v {
                Value::Ref(p) => interp.memory.get(p),
                v => v,
            };
            match v {
                Value::Struct(_, fields) => match fields.as_slice() {
                    [(_, Value::Int(n, _))] => *n as usize,
                    _ => unsound("a `ReadFile` without its number"),
                },
                v => unsound(&format!("`{v}` as a `ReadFile`")),
            }
        };
        match (op, args.as_slice()) {
            (ReadFileOp::Read, [file, buf]) => {
                let file = number(self, file);
                let Value::Ref(
                    p @ Ptr {
                        range: Some((lo, len)),
                        ..
                    },
                ) = buf
                else {
                    unsound("`os_read` into something that isn't a slice")
                };
                let (p, lo, len) = (p.clone(), *lo, *len);
                let mut bytes = vec![0; len];
                let n = match self.term.read_file(file, &mut bytes) {
                    Ok(n) => n.min(len),
                    Err(e) => return Value::Int(i128::from(io_failure(&e)), IntTy::U64),
                };
                let Value::Slots(slots) = self.memory.get_mut(&p) else {
                    unsound("a slice that isn't of slots")
                };
                for (i, &b) in bytes[..n].iter().enumerate() {
                    slots[lo + i] = Value::Int(i128::from(b), IntTy::U8);
                }
                Value::Int(n as i128, IntTy::U64)
            }
            (ReadFileOp::Close, [file]) => {
                let file = number(self, file);
                self.term.close(file);
                Value::Unit
            }
            _ => unsound(&format!("`ReadFile`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// A method of `std::env::Args`, with arguments `args`.
    fn args(&mut self, op: ArgsOp, args: Vec<Value>) -> Value {
        let all = self.term.args();
        match (op, args.as_slice()) {
            (ArgsOp::Program, [_]) => {
                let s = all[0].clone();
                Value::Ref(self.literal(s.as_bytes()))
            }
            (ArgsOp::Len, [_]) => self.usize_of(all.len() - 1),
            (ArgsOp::Index, [_, i]) => {
                let i = usize::try_from(int(i.clone())).unwrap_or_else(|_| unsound("an index"));
                let s = (all.get(i + 1).cloned())
                    .unwrap_or_else(|| unsound("an argument past the last"));
                Value::Ref(self.literal(s.as_bytes()))
            }
            _ => unsound(&format!("`Args`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// A method of `std::env::Env`, whose copy is `copy`, with arguments
    /// `args`.
    fn env(&mut self, op: EnvOp, copy: &str, args: Vec<Value>) -> Value {
        match (op, args.as_slice()) {
            (EnvOp::Len, [_]) => self.usize_of(self.term.env().len()),
            (EnvOp::Index, [_, i]) => {
                let i = usize::try_from(int(i.clone())).unwrap_or_else(|_| unsound("an index"));
                let (name, value) = (self.term.env().get(i).cloned())
                    .unwrap_or_else(|| unsound("a variable past the last"));
                let TypeExprKind::Named(var, _) = &self.fns[copy].ret.kind else {
                    unsound("`Env::index` gives a `Var`")
                };
                let var = var.clone();
                let name = Value::Ref(self.literal(name.as_bytes()));
                let value = Value::Ref(self.literal(value.as_bytes()));
                Value::Struct(var, vec![("name".into(), name), ("value".into(), value)])
            }
            _ => unsound(&format!("`Env`'s `{op:?}` of the wrong arguments")),
        }
    }

    /// Where static `symbol` is, used, as it is, by a function that says
    /// `with global`, as must all those running.
    fn static_place(&mut self, symbol: &str) -> Ptr {
        self.effect("global");
        if let Some(p) = self.statics.get(symbol) {
            return p.clone();
        }
        let value = self
            .call(
                Value::Fn(symbol.to_string(), None),
                Vec::new(),
                Span::new(0, 0),
            )
            .unwrap_or_else(|_| unsound(&format!("static `{symbol}`'s value failed")));
        let p = self.memory.alloc(value, false);
        self.statics.insert(symbol.to_string(), p.clone());
        p
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

    /// Runs an array's intrinsic method `op` on `args`: the first a
    /// reference to its values, all of them.
    fn array(&mut self, op: ArrayOp, args: Vec<Value>) -> Value {
        let name = Intrinsic::Array(op).name();
        let this = args
            .into_iter()
            .next()
            .unwrap_or_else(|| unsound("array arity"));
        let Value::Ref(p @ Ptr { range: None, .. }) = &this else {
            unsound(&format!("`{name}` of `{this}`, which isn't an array"))
        };
        let Value::Slots(values) = self.memory.get(p) else {
            unsound(&format!("`{name}` of `{this}`, which isn't an array"))
        };
        let len = values.len();
        match op {
            ArrayOp::Len => self.usize_of(len),
            ArrayOp::AsSlice | ArrayOp::AsMutSlice => Value::Ref(p.slice(0, len)),
        }
    }

    /// Runs a slice's intrinsic method `op`, whose copy is `copy`, on
    /// `args`. Each checks what it's given against the slice's length, so
    /// none can fail: it's what it says in `core/slice.t`.
    fn slice(&mut self, op: SliceOp, copy: &str, args: Vec<Value>) -> Value {
        let name = Intrinsic::Slice(op).name();
        let mut args = args.into_iter();
        let this = args.next().unwrap_or_else(|| unsound("slice arity"));
        let Value::Ref(
            p @ Ptr {
                range: Some((lo, len)),
                ..
            },
        ) = &this
        else {
            unsound(&format!("`{name}` of `{this}`, which isn't a slice"))
        };
        let (p, lo, len) = (p.clone(), *lo, *len);
        // The copy of `Option` it returns.
        let option = || -> String {
            let TypeExprKind::Named(option, _) = &self.fns[copy].ret.kind else {
                unsound(&format!("`{name}` returns an `Option`"))
            };
            option.clone()
        };
        let index = |v: Option<Value>, max: usize| {
            let i = int(v.unwrap_or_else(|| unsound("slice arity")));
            usize::try_from(i).ok().filter(|i| *i <= max)
        };
        let made = match op {
            SliceOp::Len => return self.usize_of(len),
            // Its refinement says `i < len()`, so it's the checker's bug
            // if it isn't.
            SliceOp::Index | SliceOp::IndexMut => {
                let Some(i) = index(args.next(), len).filter(|i| *i < len) else {
                    unsound(&format!("`{name}` out of range of a slice of {len}"))
                };
                return Value::Ref(Ptr {
                    range: None,
                    ..p.field(lo + i)
                });
            }
            SliceOp::Get | SliceOp::GetMut => {
                let i = index(args.next(), len).filter(|i| *i < len);
                i.map(|i| {
                    Value::Ref(Ptr {
                        range: None,
                        ..p.field(lo + i)
                    })
                })
            }
            // Its refinement says `mid <= len()`.
            SliceOp::Cut | SliceOp::CutMut => {
                let Some(mid) = index(args.next(), len) else {
                    unsound(&format!("`{name}` out of range of a slice of {len}"))
                };
                let TypeExprKind::Named(split, _) = &self.fns[copy].ret.unrefined().kind else {
                    unsound(&format!("`{name}` returns a struct"))
                };
                let head = Value::Ref(p.slice(lo, mid));
                let tail = Value::Ref(p.slice(lo + mid, len - mid));
                let fields = vec![("head".into(), head), ("tail".into(), tail)];
                return Value::Struct(split.clone(), fields);
            }
        };
        let option = option();
        match made {
            Some(v) => Value::Variant(option, "Some".into(), vec![v]),
            None => Value::Variant(option, "None".into(), Vec::new()),
        }
    }

    /// Runs `Unsafe::as_mut` or `as_ref` on `args`, a `&Unsafe<T>`: a
    /// reference to its one field, what's in it. That nothing else reaches
    /// it while the reference lives is the caller's obligation (`unsafe`,
    /// lemma U5 of docs/46-unsafe-cell.md).
    fn unsafe_cell(&mut self, args: Vec<Value>) -> Value {
        let [Value::Ref(p)] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"))
        else {
            unsound("`Unsafe::as_mut` of what isn't a reference")
        };
        if !self.is_unsafe_cell(&p) {
            unsound(&format!(
                "`Unsafe::as_mut` of `{}`, which isn't an `Unsafe`",
                self.memory.get(&p)
            ));
        }
        Value::Ref(p.field(0))
    }

    /// Is what `p` points to an `Unsafe`: a struct of type
    /// `core::cell::Unsafe::<..>`, by its copy's name?
    fn is_unsafe_cell(&self, p: &Ptr) -> bool {
        matches!(self.memory.get(p), Value::Struct(ty, _)
            if ty.split("::<").next() == Some(crate::check::UNSAFE))
    }

    /// Runs `Atomic`'s intrinsic method `op` on `args`: a `&Atomic<T>`,
    /// and its `T`s. An atomic is a struct whose one field is an `Unsafe`
    /// of what's in it, written here through a `&`, which only what's in an
    /// `Unsafe` may be. There's one thread, so each is one step anyway: the
    /// orderings are a C compiler's business (lemma Y7 of
    /// docs/38-statics.md).
    fn atomic(&mut self, op: AtomicOp, args: Vec<Value>) -> Value {
        let name = Intrinsic::Atomic(op).name();
        let mut args = args.into_iter();
        let Some(Value::Ref(p)) = args.next() else {
            unsound(&format!("`Atomic::{name}` of what isn't a reference"))
        };
        // An atomic is a struct of an `Unsafe` of what's in it.
        let cell = p.field(0);
        if !self.is_unsafe_cell(&cell) {
            unsound(&format!(
                "`Atomic::{name}` of `{}`, which isn't an atomic",
                self.memory.get(&p)
            ));
        }
        let at = cell.field(0);
        let old = self.memory.get(&at).clone();
        let args: Vec<Value> = args.collect();
        let int = |v: &Value| match v {
            Value::Int(n, t) => (*n, *t),
            _ => unsound(&format!(
                "`Atomic::{name}` of `{v}`, which isn't an integer"
            )),
        };
        let new = match (op, args.as_slice()) {
            (AtomicOp::Load, []) => return old,
            (AtomicOp::Swap, [v]) => v.clone(),
            (AtomicOp::CompareSwap, [current, new]) => match old == *current {
                true => new.clone(),
                false => return old,
            },
            (AtomicOp::FetchAnd | AtomicOp::FetchOr | AtomicOp::FetchXor, [Value::Bool(b)]) => {
                let Value::Bool(a) = old else {
                    unsound("`Atomic<bool>` of what isn't a `bool`")
                };
                Value::Bool(match op {
                    AtomicOp::FetchAnd => a & b,
                    AtomicOp::FetchOr => a | b,
                    _ => a ^ b,
                })
            }
            (
                AtomicOp::FetchAnd
                | AtomicOp::FetchOr
                | AtomicOp::FetchXor
                | AtomicOp::FetchAdd
                | AtomicOp::FetchSub,
                [v],
            ) => {
                let ((a, t), (b, u)) = (int(&old), int(v));
                if t != u {
                    unsound("`Atomic` of two integer types");
                }
                let n = match op {
                    AtomicOp::FetchAnd => a & b,
                    AtomicOp::FetchOr => a | b,
                    AtomicOp::FetchXor => a ^ b,
                    AtomicOp::FetchAdd => self.wrap(a + b, t),
                    _ => self.wrap(a - b, t),
                };
                self.int_of(n, t)
            }
            _ => unsound(&format!("`Atomic::{name}` of {} arguments", args.len())),
        };
        if self.taken_apart(&old) {
            unsound("an atomic whose value was moved out");
        }
        *self.memory.get_mut(&at) = new;
        old
    }

    /// Runs an arena's `Slots`' intrinsic method `op` on `args`. Each value put in
    /// gets a heap slot of its own, which nothing frees before the arena
    /// is dropped, so a reference `alloc` hands out stays good until then
    /// (lemmas R1 and R2 of docs/23-arenas.md), and dangles after, which
    /// the generations of slots catch.
    fn arena(&mut self, op: SlotsOp, args: Vec<Value>) -> Value {
        let name = Intrinsic::Slots(op).name();
        // Where the boxes of the arena that `self`, a `&Arena`, points to are.
        let boxes = |this: &Interp, v: &Value| -> Ptr {
            let Value::Ref(p) = v else {
                unsound(&format!("`Slots::{name}` of `{v}`"))
            };
            match this.memory.get(p) {
                Value::Arena(q) => q.clone(),
                other => unsound(&format!("`Slots::{name}` of `{other}`")),
            }
        };
        let mut args = args.into_iter();
        let mut arg = || args.next().unwrap_or_else(|| unsound("`Arena` arity"));
        match op {
            SlotsOp::New => Value::Arena(self.memory.alloc(Value::Slots(Vec::new()), true)),
            SlotsOp::Alloc => {
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
            SlotsOp::Len => {
                let q = boxes(self, &arg());
                match self.memory.get(&q) {
                    Value::Slots(values) => self.usize_of(values.len()),
                    other => unsound(&format!("an `Arena` of `{other}`")),
                }
            }
        }
    }

    /// Runs `RawBuf`'s intrinsic method `op` on `args`, called at `span`.
    /// Its slots are a `Slots` on the heap, so a reference to one is into
    /// that, and `grow` and `free` leave it dangling, as they would a
    /// compiled program's. Each unsafe one checks its obligations, as
    /// `alloc/raw.t` says them, and stops the program if they're broken,
    /// blaming the `unsafe` block that said they were kept.
    fn raw_buf(&mut self, op: RawBufOp, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        let name = Intrinsic::RawBuf(op).name();
        /// More slots than this is running out of memory.
        const MAX_SLOTS: i128 = 1 << 20;
        // The buffer that `self`, a `&RawBuf` or `&mut RawBuf`, points to.
        let buf = |this: &Interp, v: &Value| -> Ptr {
            let Value::Ref(p) = v else {
                unsound(&format!("`RawBuf::{name}` of `{v}`"))
            };
            match this.memory.get(p) {
                Value::RawBuf(q) => q.clone(),
                other => unsound(&format!("`RawBuf::{name}` of `{other}`")),
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
            RawBufOp::New => {
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
            RawBufOp::Empty => Ok(Value::RawBuf(
                self.memory.alloc(Value::Slots(Vec::new()), true),
            )),
            RawBufOp::Cap => {
                let q = buf(self, &arg());
                Ok(self.usize_of(slots(self, &q)))
            }
            RawBufOp::Grow => {
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
            RawBufOp::Slice | RawBufOp::SliceMut => {
                let block = self.unsafe_op();
                let q = buf(self, &arg());
                let (lo, hi) = (int(arg()), int(arg()));
                let cap = slots(self, &q);
                let what = format!("`RawBuf::{name}` of slots {lo}..{hi}");
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
            RawBufOp::Write | RawBufOp::Read => {
                let block = self.unsafe_op();
                let this = arg();
                let (q, i) = (buf(self, &this), int(arg()));
                let cap = slots(self, &q);
                let what = format!("`RawBuf::{name}` of slot {i}");
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
                    (RawBufOp::Write, true) => {
                        Err(self.broken(span, block, format!("{what}, which is initialised")))
                    }
                    (RawBufOp::Write, false) => {
                        *self.memory.get_mut(&slot) = arg();
                        Ok(Value::Unit)
                    }
                    (_, false) => {
                        Err(self.broken(span, block, format!("{what}, which is uninitialised")))
                    }
                    _ => Ok(std::mem::replace(self.memory.get_mut(&slot), Value::Moved)),
                }
            }
            RawBufOp::Free => {
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
            if let Some(r) = self.type_refined(&f.ty, &fields[..j], false) {
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
        let (lo, hi) = self.range(t);
        let n = match op {
            // Bit by bit, which `i128`'s operators are, on two's
            // complement: the result is in `t` (`int_of` checks).
            BinOp::BitAnd => return Ok(self.int_of(l & r, t)),
            BinOp::BitOr => return Ok(self.int_of(l | r, t)),
            BinOp::BitXor => return Ok(self.int_of(l ^ r, t)),
            // Rounding down, as `i128`'s `>>` does. Past 126 bits, every
            // integer of any type is 0, or -1 if it's negative.
            BinOp::Shr => return Ok(self.int_of(l >> r.min(127), t)),
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
