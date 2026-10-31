//! A tree-walking interpreter. It gives the language its meaning (its
//! *dynamic semantics*) so we can check that the type system is right before
//! there's a real backend.
//!
//! Look at how often this file calls `unsound(..)`: every place where a value
//! could have the "wrong shape", like adding a bool, calling an int, an `if`
//! on a number, or a `match` where no arm fits. The type checker promises those places are unreachable for
//! every program it accepts. That promise is the *type soundness* theorem
//! ("well-typed programs don't go wrong"), and those calls are where you'd see
//! it fail if the checker had a bug.
//!
//! Since step 5 it also keeps memory the way a compiled program would, so
//! that the borrow checker's promises can fail here too. Every variable and
//! temporary lives in a *slot*, freed when it goes out of scope; a box's
//! value lives in a slot on the *heap*, freed when the box is dropped. A
//! reference is the address of a slot and the path to a field in it, so
//! following one to a slot that was freed is a *dangling reference*. Moving
//! a value that owns a box out of a place leaves `Value::Moved` behind, so
//! using it again is a *use after move*, and freeing the box twice a
//! *double free*. And at the end, the heap must be empty: anything left is
//! a *leak*. The borrow checker promises none of those happen
//! (docs/05-ownership.md).
//!
//! Since step 6a.1 it keeps linear values the same way. What moves and
//! what's copied, and which patterns use up what they take apart, is what
//! the borrow checker decided, by the types it knew, as `lift` wrote it
//! out (`ExprKind::Move`, `PatternKind::Move`): in generic code, a value of
//! a type parameter that isn't known to be `Copy` moves, even where it's an
//! `i64`, so that a body means the same at every type. The types
//! monomorphisation made, which say which are `Copy` and which `Discard`,
//! only check those decisions: a copy of a value whose type isn't `Copy`,
//! or a value taken apart by value and not used up whose type isn't
//! `Discard`, is a checker bug. Throwing away whole a value that isn't
//! `Discard`, rather than using it up, is a leak too
//! (docs/06a.1-linear-types.md). Except on the way out of a run-time error:
//! that's a panic, which aborts, so nothing is really dropped.
//!
//! `return`, `break` and `continue` unwind: each is a `Flow` that runs back
//! up through the expressions it's in, ending their statements and scopes
//! on the way, to the call or loop it goes to. What those expressions had
//! worked out so far is dropped, as a compiled program would: in
//! `f(Box::new(1), return 2)`, the box.

use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::IntTy;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// Deep enough for any reasonable recursion, shallow enough to stop runaway
/// recursion with an error instead of crashing the interpreter itself.
const MAX_CALL_DEPTH: usize = 10_000;

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
    /// under their symbols (see `lift.rs`).
    Fn(String),
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

/// What `|x| x + n` evaluates to: its code, and what it captured, its
/// *environment*: for each variable, a reference to it, or its value,
/// moved or copied in, as `Lambda::modes` says (see
/// docs/06c-closures.md). Like a struct's fields, as that's what it is.
#[derive(Debug, Clone)]
pub struct Closure {
    pub code: Arc<Lambda>,
    pub env: Vec<(String, Value)>,
}

/// The same closure: made by the same expression, in equal environments. T
/// programs can't compare functions; this is for tests.
impl PartialEq for Closure {
    fn eq(&self, other: &Closure) -> bool {
        Arc::ptr_eq(&self.code, &other.code) && self.env == other.env
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n, _) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Unit => f.write_str("()"),
            Value::Fn(name) => write!(f, "<fn {}>", crate::check::source_name(name)),
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
        locals: Vec::new(),
        memory,
        temps: Vec::new(),
        depth: 0,
        frames: Vec::new(),
        unsafe_blocks: Vec::new(),
        proved: &prog.proved,
        measures: &prog.measures,
        refined: HashMap::new(),
        params_refined: Vec::new(),
        term,
    };
    // `main` takes nothing, or the one `std::Os` there is.
    let args = match interp.fns["main"].params.len() {
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
    let result = interp.call(Value::Fn("main".to_string()), args, Span::new(0, 0))?;
    // An error it gives back is reported, by the copy of `std::rt::report`
    // monomorphisation made for it, with its own `Stderr`.
    if let Value::Variant(ty, variant, fields) = &result
        && ty.starts_with("core::result::Result::<")
        && variant == "Err"
    {
        let report = (interp.fns.keys())
            .find(|f| f.starts_with("std::rt::report::<"))
            .unwrap_or_else(|| unsound("no `std::rt::report` for `main`'s error"))
            .to_string();
        let [e] = fields.as_slice() else {
            unsound("an `Err` of one field")
        };
        let at = interp.memory.alloc(e.clone(), false);
        let stderr = Value::Struct("std::io::Stderr".into(), Vec::new());
        interp.call(
            Value::Fn(report),
            vec![Value::Ref(at.clone()), stderr],
            Span::new(0, 0),
        )?;
        interp.memory.free(&at, false);
    }
    interp.drop(result.clone())?;
    if interp.memory.on_heap > 0 {
        unsound(&format!(
            "memory leak: {} boxes or buffers were never freed",
            interp.memory.on_heap
        ));
    }
    Ok(result)
}

/// Why evaluating an expression didn't finish with a value: an error, or a
/// jump to somewhere outside it, with the value it takes there, and for
/// `break` and `continue`, the label of the loop it's for, if it says.
enum Flow {
    Error(Error),
    Return(Value),
    Break(Option<String>, Value),
    Continue(Option<String>),
}

impl From<Error> for Flow {
    fn from(e: Error) -> Flow {
        Flow::Error(e)
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
    /// The variables in scope, innermost last, each with its slot. A
    /// temporary that a `let` keeps alive (`let r = &f();`) is one with no
    /// name.
    /// Whether each owns its slot: not a variable a closure captured, in
    /// its body, which is where the closure says (see `call_closure`).
    locals: Vec<(String, Ptr, bool)>,
    memory: Memory,
    /// The temporaries made in each statement being run, innermost last,
    /// to free when it's done.
    temps: Vec<Vec<Ptr>>,
    depth: usize,
    /// The functions being run, outermost first, by name: whose effects
    /// each panic and allocation must be among (see `effect`).
    frames: Vec<String>,
    /// The `unsafe` blocks being run, innermost last, each with how many
    /// functions were being run when it started: those called since must
    /// say `with unsafe` (see `unsafe_op`).
    unsafe_blocks: Vec<(Span, usize)>,
    /// The operators the checker proved can't panic, by node.
    proved: &'p HashSet<NodeId>,
    /// What the checker found of measures, for refinements that use them.
    measures: &'p crate::ast::Measures,
    /// The refinement of each refined variable, by its slot and the
    /// slot's generation: what each assignment to it must keep.
    refined: HashMap<(usize, u64), Refined>,
    /// The refinements of the parameters of the function about to be
    /// entered, by name, for `enter` to keep in `refined`.
    params_refined: Vec<(String, Refined)>,
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
            _ => unsound("a refinement outside the logic"),
        }
    }
}

/// Each measure call in predicate `e`, by node, with its receiver.
fn each_measure_call<'e>(e: &'e Expr, out: &mut Vec<(NodeId, &'e Expr)>) {
    if let ExprKind::MethodCall(x, ..) = &e.kind {
        out.push((e.id, x));
    }
    for c in e.children() {
        each_measure_call(c, out);
    }
}

/// The names in predicate `e` other than `binder`.
fn pred_names<'e>(e: &'e Expr, binder: &str, out: &mut Vec<&'e str>) {
    if let ExprKind::Var(name, _) = &e.kind
        && name != binder
        && !out.contains(&name.as_str())
    {
        out.push(name);
    }
    for c in e.children() {
        pred_names(c, binder, out);
    }
}

impl Interp<'_, '_> {
    /// The refinement `{ binder | pred }`, whose other names have the
    /// values `env`: what the measures it calls of them give is found now.
    fn refined(&self, binder: &str, pred: &Expr, env: Vec<(String, Value)>) -> Refined {
        let mut r = Refined {
            binder: binder.to_string(),
            pred: pred.clone(),
            env,
            measured: HashMap::new(),
        };
        let mut calls = Vec::new();
        each_measure_call(pred, &mut calls);
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
                ("alloc::raw::RawBuf::cap", this) => match self.pred_deref(this.clone()) {
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

    /// The slot of local variable `name`, if it's one.
    fn local(&self, name: &str) -> Option<Ptr> {
        let local = self.locals.iter().rev().find(|(n, ..)| n == name);
        local.map(|(_, p, _)| p.clone())
    }

    /// The function `name` names, which isn't a local.
    fn function(&self, name: &str) -> Value {
        if self.fns.contains_key(name) {
            return Value::Fn(name.to_string());
        }
        unsound(&format!("unbound variable `{name}`"))
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

    /// Frees everything `v` owns: the boxes in it, and what's in those.
    ///
    /// Throwing away a struct or enum value that isn't `Discard` is a leak,
    /// unless it's been taken apart, by moving out of it: then only what's
    /// left in it counts, each part on its own, as for the borrow checker
    /// (`moves::leftover`).
    ///
    /// A struct or enum with a `discard` of its own, a function `<T as
    /// Discard>::discard` for its type `T`, is dropped by calling that,
    /// unless it's been taken apart, or the program is aborting. That can
    /// fail only by running out of stack: `discard` is total.
    fn drop(&mut self, v: Value) -> Result<(), Error> {
        if let Value::RawBuf(_) = v
            && !self.aborting
        {
            unsound("a `RawBuf` was thrown away, and it isn't `Discard`: a leak");
        }
        if let Value::Struct(ty, _) | Value::Variant(ty, ..) = &v
            && !self.aborting
            && !self.discard.contains(ty.as_str())
            && !self.taken_apart(&v)
        {
            unsound(&format!(
                "a `{}` was thrown away, and it isn't `Discard`: a leak",
                crate::check::source_name(ty)
            ));
        }
        if let Value::Struct(ty, _) | Value::Variant(ty, ..) = &v
            && !self.aborting
            && !self.taken_apart(&v)
            && let Some(f) = self.user_discard(ty)
        {
            return self
                .call(Value::Fn(f), vec![v], Span::new(0, 0))
                .map(|_| ());
        }
        self.drop_parts(v)
    }

    /// Drops what `v` holds, but not `v` itself: the parts that are left,
    /// in order.
    fn drop_parts(&mut self, v: Value) -> Result<(), Error> {
        let parts: Vec<Value> = match v {
            Value::Box(p) => vec![self.memory.free(&p, true)],
            // Its values' boxes, first put in first.
            Value::Arena(p) => vec![self.memory.free(&p, true)],
            Value::Struct(_, fields) => fields.into_iter().map(|(_, v)| v).collect(),
            Value::Variant(_, _, fields) => fields,
            Value::Closure(c) => c.env.into_iter().map(|(_, v)| v).collect(),
            // An array's values, front to back.
            Value::Slots(values) => values,
            _ => Vec::new(),
        };
        self.drop_all(parts)
    }

    /// Drops each of `vs`, in order, all of them even if one fails, as
    /// what's left must be freed anyway; the first failure is the result.
    fn drop_all(&mut self, vs: impl IntoIterator<Item = Value>) -> Result<(), Error> {
        let mut result = Ok(());
        for v in vs {
            let r = self.drop(v);
            if result.is_ok() {
                result = r;
            }
        }
        result
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
        Error::runtime(span, message)
    }

    /// A panic, which every function being run must say it may have: if
    /// one doesn't, the checker let through what it shouldn't have
    /// (see docs/07-effects.md). Not running out of stack, which no effect
    /// says.
    fn panics(&mut self, span: Span, message: impl Into<String>) -> Error {
        self.effect("panic");
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

    /// Drops the variables from `scope` on, the last declared first, and
    /// frees their slots.
    ///
    /// In a type's own `discard`, `self` is dropped part by part, as the
    /// borrow checker has it (lemma D3 of docs/20-destructors.md): its
    /// `discard` is what's running.
    fn end_scope(&mut self, scope: usize) -> Result<(), Error> {
        let discarding = self.frames.last().is_some_and(|f| is_discard_fn(f));
        let mut result = Ok(());
        while self.locals.len() > scope {
            let (name, p, owned) = self.locals.pop().expect("a local");
            if !owned {
                continue;
            }
            let v = self.memory.free(&p, false);
            let r = match discarding && self.locals.is_empty() && name == "self" {
                true => self.drop_parts(v),
                false => self.drop(v),
            };
            if result.is_ok() {
                result = r;
            }
        }
        result
    }

    /// Runs `f` as a statement: the temporaries it makes are dropped, in
    /// reverse, when it's done.
    fn statement<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, Flow>) -> Result<T, Flow> {
        self.temps.push(Vec::new());
        let result = f(self);
        let temps: Vec<Value> = (self.temps.pop().expect("pushed").into_iter().rev())
            .map(|p| self.memory.free(&p, false))
            .collect();
        let dropped = self.drop_all(temps);
        let v = result?;
        dropped?;
        Ok(v)
    }

    /// A new temporary holding `v`, dropped at the end of the statement.
    fn temp(&mut self, v: Value) -> Ptr {
        let p = self.memory.alloc(v, false);
        match self.temps.last_mut() {
            Some(temps) => temps.push(p.clone()),
            None => unsound("a temporary outside any statement"),
        }
        p
    }

    /// Where the value of place expression `e` is: where its root is,
    /// then its path (see `root`).
    fn place(&mut self, e: &Expr) -> Result<Ptr, Flow> {
        let base = self.base(root(e))?;
        Ok(self.path(e, &base))
    }

    /// Where the value of `root`, the root of a place expression, is: a
    /// variable's slot, or else a new temporary, holding its value.
    fn base(&mut self, root: &Expr) -> Result<Ptr, Flow> {
        if let ExprKind::Var(name, _) = &root.kind {
            if let Some(p) = self.local(name) {
                return Ok(p);
            }
            let f = self.function(name);
            return Ok(self.temp(f));
        }
        let v = self.eval(root)?;
        Ok(self.temp(v))
    }

    /// Where the value of place expression `e` is, given `base`, where its
    /// root is: following its fields and `*`s, which does nothing but read
    /// memory, so can't fail, nor jump anywhere.
    fn path(&self, e: &Expr, base: &Ptr) -> Ptr {
        match &e.kind {
            // Through any references and boxes to the struct.
            ExprKind::Field(x, field) => {
                let mut p = self.path(x, base);
                loop {
                    match self.memory.get(&p) {
                        Value::Ref(q) | Value::Box(q) => p = q.clone(),
                        v @ Value::Struct(_, fields) => {
                            self.check_fields(v, "read");
                            let i = fields.iter().position(|(f, _)| *f == field.name);
                            let i =
                                i.unwrap_or_else(|| unsound(&format!("no field `{}`", field.name)));
                            return p.field(i);
                        }
                        other => unsound(&format!("field access on `{other}`")),
                    }
                }
            }
            ExprKind::Unary(UnOp::Deref, x) => {
                let p = self.path(x, base);
                match self.memory.get(&p) {
                    Value::Ref(q) | Value::Box(q) => q.clone(),
                    other => unsound(&format!("`*` on `{other}`")),
                }
            }
            _ => base.clone(),
        }
    }

    fn call(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        match f {
            Value::Fn(name) => {
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
                let intrinsic = decl.intrinsic.is_some();
                for (p, v) in decl.params.iter().zip(&args) {
                    if let Some(r) = refinement(self, &p.ty) {
                        r.check(
                            self,
                            v,
                            &format!("argument `{}` of `{}`", p.name.name, decl.name.name),
                        );
                        if !intrinsic {
                            self.params_refined.push((p.name.name.clone(), r));
                        }
                    }
                }
                let ret = refinement(self, &decl.ret);
                // What an intrinsic gives is trusted, and so checked: its
                // refinement too.
                let result = if intrinsic {
                    let of = name.split("::<").next().unwrap_or_default().to_string();
                    self.intrinsic(&of, &name, args, span)
                } else {
                    // A call gets a fresh environment: just the parameters.
                    let params = decl.params.iter().map(|p| p.name.name.clone());
                    self.frames.push(name);
                    let result =
                        self.enter(Vec::new(), params.zip(args).collect(), &decl.body, span);
                    self.frames.pop();
                    result
                };
                if let (Ok(v), Some(r)) = (&result, ret) {
                    r.check(self, v, &format!("the result of `{}`", decl.name.name));
                }
                result
            }
            // A closure called by value is the call's to use up, and what's
            // left of it is dropped when it returns.
            Value::Closure(c) => {
                let at = self.memory.alloc(Value::Closure(c), false);
                let result = self.call_closure(&at, args, span);
                let left = self.memory.free(&at, false);
                let dropped = self.drop(left);
                let v = result?;
                dropped?;
                Ok(v)
            }
            // Through a reference, what it points to is called, in place.
            Value::Ref(p) => match self.memory.get(&p).clone() {
                Value::Closure(_) => self.call_closure(&p, args, span),
                f => self.call(f, args, span),
            },
            other => unsound(&format!("called a non-function `{other}`")),
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
            "core::array::from_fn" => {
                let [f] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
                let n = match &self.fns[copy].ret.kind {
                    TypeExprKind::Array(_, n) => match n.kind {
                        TypeExprKind::Nat(n) => n as usize,
                        _ => unsound("`from_fn` of an array of no length"),
                    },
                    _ => unsound("`from_fn` makes an array"),
                };
                let at = self.memory.alloc(f, false);
                let mut values = Vec::new();
                let mut result = Ok(());
                for i in 0..n {
                    match self.call(Value::Ref(at.clone()), vec![usize_of(i)], span) {
                        Ok(v) => values.push(v),
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
                let left = self.memory.free(&at, false);
                let dropped = self.drop(left);
                if result.is_err() {
                    let _ = self.drop_all(values.drain(..));
                }
                result?;
                dropped?;
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
    /// there must be one, and every function called since it started must
    /// say `with unsafe`, or the checker let through what it shouldn't
    /// have (see docs/09-unsafe.md).
    fn unsafe_op(&self) -> Span {
        let Some(&(block, depth)) = self.unsafe_blocks.last() else {
            unsound("an unsafe operation with no `unsafe` block around it")
        };
        for f in &self.frames[depth..] {
            let says = self.fns[f.as_str()]
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
        block
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

    /// Calls the closure at `at`. In its body, each variable it captured is
    /// where the closure says: what the reference it holds points to, if it
    /// captured it by reference, or else its own field.
    fn call_closure(&mut self, at: &Ptr, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        let Value::Closure(c) = self.memory.get(at).clone() else {
            unreachable!("a closure")
        };
        let mut captured = Vec::new();
        for (i, (name, mode, _)) in c.code.modes.iter().enumerate() {
            let p = match mode {
                Capture::Value => at.field(i),
                Capture::Ref | Capture::Mut | Capture::Unique => {
                    match self.memory.get(&at.field(i)) {
                        Value::Ref(p) => p.clone(),
                        other => unsound(&format!("a closure holding `{other}` for a reference")),
                    }
                }
            };
            captured.push((name.clone(), p));
        }
        let params = c.code.params.iter().map(|p| p.name.name.clone());
        self.enter(captured, params.zip(args).collect(), &c.code.body, span)
    }

    /// Evaluates a function body with `locals` as its environment, each in
    /// a new slot, freed at the end, and in a closure's, what it captured,
    /// `captured`, where it is. The caller's locals are saved and restored
    /// afterwards.
    fn enter(
        &mut self,
        captured: Vec<(String, Ptr)>,
        locals: Vec<(String, Value)>,
        body: &Expr,
        span: Span,
    ) -> Result<Value, Error> {
        if self.depth >= MAX_CALL_DEPTH {
            return Err(self.panic(
                span,
                format!("stack overflow (more than {MAX_CALL_DEPTH} nested calls)"),
            ));
        }
        let captured = captured.into_iter().map(|(name, p)| (name, p, false));
        let locals = captured
            .chain(
                locals
                    .into_iter()
                    .map(|(name, v)| (name, self.memory.alloc(v, false), true)),
            )
            .collect();
        let saved = std::mem::replace(&mut self.locals, locals);
        for (name, r) in std::mem::take(&mut self.params_refined) {
            let p = self.local(&name).expect("a parameter");
            self.refined.insert((p.slot, p.generation), r);
        }
        let saved_temps = std::mem::take(&mut self.temps);
        self.depth += 1;
        let result = self.statement(|i| i.eval(body));
        self.depth -= 1;
        let ended = self.end_scope(0);
        self.locals = saved;
        self.temps = saved_temps;
        let result = match (result, ended) {
            (Ok(_) | Err(Flow::Return(_)), Err(e)) => Err(Flow::Error(e)),
            (result, _) => result,
        };
        match result {
            Ok(v) | Err(Flow::Return(v)) => Ok(v),
            Err(Flow::Error(e)) => Err(e),
            Err(Flow::Break(..) | Flow::Continue(_)) => {
                unsound("a `break` or `continue` out of a function")
            }
        }
    }

    /// Evaluates `es` in order. If one doesn't finish, what the ones before
    /// it gave is dropped.
    fn eval_all<'e>(&mut self, es: impl IntoIterator<Item = &'e Expr>) -> Result<Vec<Value>, Flow> {
        let mut values = Vec::new();
        for e in es {
            match self.eval(e) {
                Ok(v) => values.push(v),
                Err(flow) => {
                    self.drop_all(values.into_iter().rev())?;
                    return Err(flow);
                }
            }
        }
        Ok(values)
    }

    /// A call's arguments, in order, but for borrows, `&e` or `&mut e`,
    /// which are taken after the rest, as the borrow checker says
    /// (`Builder::call_into`). Only the borrow waits, and following the
    /// place's path: its root is worked out in order, as working it out
    /// may do things. `&*make()` calls `make` where it's written.
    fn arguments<'e>(
        &mut self,
        args: impl IntoIterator<Item = &'e Expr>,
    ) -> Result<Vec<Value>, Flow> {
        let args: Vec<&Expr> = args.into_iter().collect();
        // Each argument's value, or for a borrow, where its place's root is.
        let mut firsts: Vec<Result<Value, Ptr>> = Vec::new();
        for &a in &args {
            let first = match &a.kind {
                ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) => self.base(root(x)).map(Err),
                _ => self.eval(a).map(Ok),
            };
            match first {
                Ok(first) => firsts.push(first),
                Err(flow) => {
                    self.drop_all(firsts.into_iter().rev().filter_map(Result::ok))?;
                    return Err(flow);
                }
            }
        }
        let borrow = |i: &Self, a: &Expr, base: Ptr| match &a.kind {
            ExprKind::Unary(_, x) => Value::Ref(i.path(x, &base)),
            _ => unreachable!("a borrow"),
        };
        Ok(args
            .iter()
            .zip(firsts)
            .map(|(a, first)| first.unwrap_or_else(|base| borrow(self, a, base)))
            .collect())
    }

    /// Runs the body of a loop again and again, until a `break`, whose
    /// value it returns, or for a `while`, until its condition `c` is false.
    ///
    /// A `break` or `continue` with another loop's label goes on out, to
    /// that loop.
    fn repeat(
        &mut self,
        label: &Option<Label>,
        c: Option<&Expr>,
        body: &Expr,
    ) -> Result<Value, Flow> {
        let ours = |l: &Option<String>| match (l, label) {
            (None, _) => true,
            (Some(l), Some(label)) => *l == label.name.name,
            (Some(_), None) => false,
        };
        loop {
            if let Some(c) = c
                && !bool(self.statement(|i| i.eval(c))?)
            {
                return Ok(Value::Unit);
            }
            match self.eval(body) {
                Ok(v) => self.drop(v)?,
                Err(Flow::Break(l, v)) if ours(&l) => return Ok(v),
                Err(Flow::Continue(l)) if ours(&l) => {}
                Err(flow) => return Err(flow),
            }
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Flow> {
        Ok(match &e.kind {
            ExprKind::For(_)
            | ExprKind::IfLet(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Range(..)
            | ExprKind::Try(..)
            | ExprKind::Index(..) => {
                unreachable!("rewritten when paths are resolved")
            }
            ExprKind::Int(n, t) => {
                let t = t.unwrap_or_else(|| unsound("a literal of no type"));
                int_of(i128::from(*n), t)
            }
            // `-` of a literal is a literal: `-128` is an `i8`, though `128`
            // isn't.
            ExprKind::Unary(UnOp::Neg, x) if let ExprKind::Int(n, t) = x.kind => {
                let t = t.unwrap_or_else(|| unsound("a literal of no type"));
                int_of(-i128::from(n), t)
            }
            ExprKind::Char(c) => Value::Char(*c),
            ExprKind::Str(s) => Value::Ref(self.literal(s.as_bytes())),
            ExprKind::ByteStr(b) => Value::Ref(self.literal(b)),
            // A character's scalar value; a number as a character, if it's
            // one.
            ExprKind::As(x, t) if matches!(&t.kind, TypeExprKind::Named(n, _) if n == "char") => {
                let n = match self.eval(x)? {
                    Value::Char(c) => i128::from(u32::from(c)),
                    v => int(v),
                };
                match u32::try_from(n).ok().and_then(char::from_u32) {
                    Some(c) => Value::Char(c),
                    None => {
                        self.not_proved(e.id);
                        let message = format!("`{n}` isn't a character");
                        return Err(self.panics(e.span, message).into());
                    }
                }
            }
            // The same number, if it fits.
            ExprKind::As(x, t) => {
                let n = match self.eval(x)? {
                    Value::Char(c) => i128::from(u32::from(c)),
                    v => int(v),
                };
                let t = int_ty(t);
                let (lo, hi) = t.range_64();
                if !(lo..=hi).contains(&n) {
                    self.not_proved(e.id);
                    let message = format!("`as` overflowed: {n} isn't {}", t.a());
                    return Err(self.panics(e.span, message).into());
                }
                Value::Int(n, t)
            }
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Unit => Value::Unit,
            ExprKind::Var(name, _) => match self.local(name) {
                Some(p) => self.read(&p, false),
                None => self.function(name),
            },
            ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _) => {
                let p = self.place(e)?;
                self.read(&p, false)
            }
            ExprKind::Move(x) => {
                let p = self.place(x)?;
                self.read(&p, true)
            }
            ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) => Value::Ref(self.place(x)?),

            ExprKind::Unary(op, x) => {
                let v = self.eval(x)?;
                match op {
                    UnOp::Neg => {
                        let zero = match v {
                            Value::Int(_, t) => Value::Int(0, t),
                            ref other => unsound(&format!("`-` of `{other}`")),
                        };
                        self.arith(BinOp::Sub, zero, v, e)?
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

            // `&&` and `||` short-circuit: the right side only runs if needed.
            ExprKind::Binary(BinOp::And, l, r) => {
                Value::Bool(bool(self.eval(l)?) && bool(self.eval(r)?))
            }
            ExprKind::Binary(BinOp::Or, l, r) => {
                Value::Bool(bool(self.eval(l)?) || bool(self.eval(r)?))
            }

            // `==` looks at its operands where they are, and moves neither.
            ExprKind::Binary(op @ (BinOp::Eq | BinOp::Ne), l, r) => {
                let l = self.place(l)?;
                let r = self.place(r)?;
                let same = self.equal(&l, &r, e.span)?;
                Value::Bool(same == (*op == BinOp::Eq))
            }

            ExprKind::Binary(op, l, r) => {
                let l = self.eval(l)?;
                let r = self.eval(r)?;
                self.arith(*op, l, r, e)?
            }

            // A function value owns nothing, so there's nothing to drop if
            // an argument doesn't finish.
            //
            // A closure's `&f` or `&mut f`, as `lift` writes a call of one
            // that's `Fn` or `FnMut`, is taken after the arguments, as any
            // borrow passed is.
            ExprKind::Call(f, args) => {
                let mut all = self.arguments(std::iter::once(&**f).chain(args))?;
                let f = all.remove(0);
                self.call(f, all, e.span)?
            }

            ExprKind::If(c, then, els) => {
                if bool(self.statement(|i| i.eval(c))?) {
                    self.eval(then)?
                } else if let Some(els) = els {
                    self.eval(els)?
                } else {
                    Value::Unit
                }
            }

            ExprKind::While(label, c, body) => self.repeat(label, Some(c), body)?,
            ExprKind::Loop(label, body) => self.repeat(label, None, body)?,
            ExprKind::Return(value) => {
                let v = match value {
                    Some(v) => self.eval(v)?,
                    None => Value::Unit,
                };
                return Err(Flow::Return(v));
            }
            ExprKind::Break(label, value) => {
                let v = match value {
                    Some(v) => self.eval(v)?,
                    None => Value::Unit,
                };
                return Err(Flow::Break(label.as_ref().map(|l| l.name.clone()), v));
            }
            ExprKind::Continue(label) => {
                return Err(Flow::Continue(label.as_ref().map(|l| l.name.clone())));
            }

            // The value first, then the place, as in Rust. What was there
            // is dropped.
            ExprKind::Assign(op, place, value) => {
                let v = self.eval(value)?;
                let p = match self.place(place) {
                    Ok(p) => p,
                    Err(flow) => {
                        self.drop(v)?;
                        return Err(flow);
                    }
                };
                let v = match op {
                    Some(op) => {
                        let old = self.memory.get(&p).clone();
                        self.arith(*op, old, v, e)?
                    }
                    None => v,
                };
                if p.path.is_empty()
                    && let Some(r) = self.refined.get(&(p.slot, p.generation))
                {
                    r.check(self, &v, "a refined variable assigned");
                }
                let old = std::mem::replace(self.memory.get_mut(&p), v);
                self.drop(old)?;
                if !p.path.is_empty() {
                    let mut outer = p.clone();
                    outer.path.pop();
                    self.check_fields(self.memory.get(&outer), "assigned to");
                }
                Value::Unit
            }

            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref())?,
            ExprKind::Unsafe(block) => {
                self.unsafe_blocks.push((e.span, self.frames.len()));
                let v = self.eval(block);
                self.unsafe_blocks.pop();
                v?
            }

            // These live in their own functions to keep `eval`'s stack frame
            // small: in debug builds every temporary in this `match` gets its
            // own slot, and deep recursion multiplies the frame size.
            ExprKind::StructLit(name, _, inits) => self.struct_lit(name, inits)?,
            ExprKind::MethodCall(..) => unreachable!("lifting makes method calls plain calls"),
            // Named by its symbol, as lifting names everything.
            ExprKind::Variant(ty, _, _, args, ..) if ty.name == crate::check::BOX => {
                let v = self.eval(&args[0])?;
                self.effect("alloc");
                Value::Box(self.memory.alloc(v, true))
            }
            ExprKind::Variant(ty, _, variant, args, ..) => {
                let args = self.eval_all(args)?;
                Value::Variant(ty.name.clone(), variant.name.clone(), args)
            }
            ExprKind::Array(items) => Value::Slots(self.eval_all(items)?),
            // `n` copies of `v`, which is `Copy`; monomorphisation wrote the
            // length `n` is.
            ExprKind::Repeat(v, n) => {
                let v = self.eval(v)?;
                let n = usize::try_from(int(self.eval(n)?))
                    .unwrap_or_else(|_| unsound("an array's length out of range"));
                Value::Slots(vec![v; n])
            }
            ExprKind::Match(scrutinee, arms) => self.match_expr(scrutinee, arms)?,
            // Only the locals the body uses: its free variables that aren't
            // functions. They're `Copy`, so they're copied.
            ExprKind::Lambda(code) => {
                let mut env = Vec::new();
                for (name, mode, moves) in &code.modes {
                    let p = self
                        .local(name)
                        .unwrap_or_else(|| unsound("capturing a non-variable"));
                    let v = match mode {
                        Capture::Value => self.read(&p, *moves),
                        Capture::Ref | Capture::Mut | Capture::Unique => Value::Ref(p),
                    };
                    env.push((name.clone(), v));
                }
                Value::Closure(Closure {
                    env,
                    code: code.clone(),
                })
            }
        })
    }

    /// A block: each statement's temporaries are dropped at its end, and
    /// the block's variables at the block's end, after the value of its
    /// tail is worked out, and the tail's temporaries dropped.
    fn block(&mut self, stmts: &[Stmt], tail: Option<&Expr>) -> Result<Value, Flow> {
        let scope = self.locals.len();
        let result = self.block_inner(stmts, tail);
        let ended = self.end_scope(scope);
        let v = result?;
        ended?;
        Ok(v)
    }

    fn block_inner(&mut self, stmts: &[Stmt], tail: Option<&Expr>) -> Result<Value, Flow> {
        for stmt in stmts {
            match stmt {
                Stmt::Let {
                    pat,
                    ty,
                    init: Some(init),
                    ..
                } => {
                    self.let_stmt(pat, init)?;
                    if let Some((v, pred)) = ty.as_ref().and_then(|t| t.refinement()) {
                        self.refine_let(pat, v, pred);
                    }
                }
                // No value yet: as if it had been moved out of.
                Stmt::Let {
                    pat, init: None, ..
                } => {
                    let PatternKind::Bind(name, _) = &pat.kind else {
                        unsound("`let` with no value of a pattern");
                    };
                    let slot = self.memory.alloc(Value::Moved, false);
                    self.locals.push((name.clone(), slot, true));
                }
                Stmt::Expr(e) | Stmt::BlockLike(e) => {
                    self.statement(|i| {
                        let v = i.eval(e)?;
                        i.drop(v)?;
                        Ok(())
                    })?;
                }
                Stmt::Fn(_) | Stmt::Type(_) => {
                    unreachable!("nested functions and types are lifted first")
                }
            }
        }
        match tail {
            Some(tail) => self.statement(|i| i.eval(tail)),
            None => Ok(Value::Unit),
        }
    }

    /// Checks that the variable `pat` has just been given has refinement
    /// `{ v: .. | pred }`, and keeps it for what's assigned to it.
    fn refine_let(&mut self, pat: &Pattern, v: &Ident, pred: &Expr) {
        let pat = match &pat.kind {
            PatternKind::Move(p) => p,
            _ => pat,
        };
        let PatternKind::Bind(name, _) = &pat.kind else {
            unsound("a refined `let` of a pattern");
        };
        let mut names = Vec::new();
        pred_names(pred, &v.name, &mut names);
        let env = names
            .into_iter()
            .map(|n| {
                let p = self
                    .local(n)
                    .unwrap_or_else(|| unsound(&format!("`{n}` isn't there")));
                (n.to_string(), self.memory.get(&p).clone())
            })
            .collect();
        let r = self.refined(&v.name, pred, env);
        let p = self.local(name).expect("just bound");
        r.check(self, self.memory.get(&p), &format!("`{name}`"));
        self.refined.insert((p.slot, p.generation), r);
    }

    /// `let pat = init;`. If `init` is `&e` of a value that isn't a place,
    /// the temporary holding it lives as long as a variable would, to the
    /// end of the block, as in Rust: `let r = &make();` is fine.
    fn let_stmt(&mut self, pat: &Pattern, init: &Expr) -> Result<(), Flow> {
        if let ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) = &init.kind
            && !x.is_place()
        {
            let v = self.statement(|i| i.eval(x))?;
            let p = self.memory.alloc(v, false);
            self.locals.push((String::new(), p.clone(), true));
            let r = self.memory.alloc(Value::Ref(p), false);
            self.temps.push(vec![r.clone()]);
            self.bind(pat, &r, false)?;
            let temps = self.temps.pop().expect("pushed");
            for p in temps {
                self.memory.free(&p, false);
            }
            return Ok(());
        }
        self.statement(|i| {
            // The value, moved or copied as `init` says, into the variable,
            // as the borrow checker has it.
            if let PatternKind::Bind(name, _) = &pat.kind {
                let v = i.eval(init)?;
                let slot = i.memory.alloc(v, false);
                i.locals.push((name.clone(), slot, true));
                return Ok(());
            }
            let p = i.place(init)?;
            if !i.matches(pat, &p) {
                unsound("a `let` pattern didn't match");
            }
            i.bind(pat, &p, false)?;
            Ok(())
        })
    }

    fn struct_lit(&mut self, name: &Ident, inits: &[FieldInit]) -> Result<Value, Flow> {
        let decl = self.structs[name.name.as_str()];
        // Evaluate in the order written, store in the order declared.
        let values = self.eval_all(inits.iter().map(|i| &i.value))?;
        let mut values: Vec<(&str, Option<Value>)> = inits
            .iter()
            .map(|i| i.name.name.as_str())
            .zip(values.into_iter().map(Some))
            .collect();
        let fields = decl
            .fields
            .iter()
            .map(|f| {
                let i = values.iter().position(|(n, _)| *n == f.name.name);
                let i = i.unwrap_or_else(|| unsound("missing struct field"));
                (f.name.name.clone(), values[i].1.take().expect("each once"))
            })
            .collect();
        let v = Value::Struct(name.name.clone(), fields);
        self.check_fields(&v, "made");
        Ok(v)
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

    /// Picks the first arm whose pattern matches, without moving anything,
    /// then binds its variables, moving or borrowing from the scrutinee.
    /// The scrutinee is looked at where it is, if it's a place.
    fn match_expr(&mut self, scrutinee: &Expr, arms: &[Arm]) -> Result<Value, Flow> {
        self.statement(|i| {
            let p = i.place(scrutinee)?;
            let Some(arm) = arms.iter().find(|arm| i.matches(&arm.pat, &p)) else {
                unsound(&format!("no `match` arm fits `{}`", i.memory.get(&p)));
            };
            let scope = i.locals.len();
            i.bind(&arm.pat, &p, false)?;
            let result = i.eval(&arm.body);
            let ended = i.end_scope(scope);
            let v = result?;
            ended?;
            Ok(v)
        })
    }

    /// Does `p` match the value at `v`? Patterns that look inside a value
    /// look through references to it, as the checker's do.
    fn matches(&self, p: &Pattern, v: &Ptr) -> bool {
        if let PatternKind::Move(p) = &p.kind {
            return self.matches(p, v);
        }
        let mut v = v.clone();
        let looks_inside = !matches!(
            p.kind,
            PatternKind::Wild | PatternKind::Bind(..) | PatternKind::Or(_)
        );
        while let (true, Value::Ref(q)) = (looks_inside, self.memory.get(&v)) {
            v = q.clone();
        }
        match (&p.kind, self.memory.get(&v)) {
            (PatternKind::Wild | PatternKind::Bind(..), _) => true,
            (PatternKind::Int(n, _), Value::Int(m, _)) => n == m,
            (PatternKind::Range(lo, hi, ..), Value::Int(m, _)) => {
                lo.is_none_or(|lo| lo <= *m) && hi.is_none_or(|hi| *m <= hi)
            }
            (PatternKind::Int(n, _), Value::Char(c)) => *n == i128::from(u32::from(*c)),
            (PatternKind::Range(lo, hi, ..), Value::Char(c)) => {
                let m = i128::from(u32::from(*c));
                lo.is_none_or(|lo| lo <= m) && hi.is_none_or(|hi| m <= hi)
            }
            (PatternKind::Bool(b), Value::Bool(c)) => b == c,
            (PatternKind::Unit, Value::Unit) => true,
            (PatternKind::Variant(_, _, variant, pats), Value::Variant(_, name, _)) => {
                variant.name == *name
                    && pats
                        .iter()
                        .enumerate()
                        .all(|(i, p)| self.matches(p, &v.field(i)))
            }
            (PatternKind::Or(alts), _) => alts.iter().any(|alt| self.matches(alt, &v)),
            (PatternKind::Array(pats), Value::Slots(values)) => {
                if pats.len() != values.len() {
                    unsound("an array pattern of another length");
                }
                (pats.iter().enumerate()).all(|(i, p)| self.matches(p, &v.field(i)))
            }
            (PatternKind::Struct(_, _, field_pats, _), Value::Struct(_, fields)) => {
                field_pats.iter().all(|fp| {
                    match fields.iter().position(|(f, _)| *f == fp.name.name) {
                        Some(i) => self.matches(&fp.pat, &v.field(i)),
                        None => unsound(&format!("no field `{}`", fp.name.name)),
                    }
                })
            }
            (_, v) => unsound(&format!("pattern can't apply to `{v}`")),
        }
    }

    /// Binds the variables of `p`, which matches the value at `v`: each to
    /// what it matches, moved out if the pattern says so, or else copied,
    /// or a reference to it if `by_ref` or the pattern looked through a
    /// reference to get there.
    fn bind(&mut self, p: &Pattern, v: &Ptr, mut by_ref: bool) -> Result<(), Error> {
        let (p, moves) = match &p.kind {
            PatternKind::Move(p) => (&**p, true),
            _ => (p, false),
        };
        let mut v = v.clone();
        let looks_inside = !matches!(
            p.kind,
            PatternKind::Wild | PatternKind::Bind(..) | PatternKind::Or(_)
        );
        while let (true, Value::Ref(q)) = (looks_inside, self.memory.get(&v)) {
            v = q.clone();
            by_ref = true;
        }
        match &p.kind {
            PatternKind::Bind(name, _) => {
                let value = match (by_ref, moves) {
                    (true, true) => unsound("a move out from behind a reference"),
                    (true, false) => Value::Ref(v),
                    (false, _) => self.read(&v, moves),
                };
                let slot = self.memory.alloc(value, false);
                self.locals.push((name.clone(), slot, true));
            }
            PatternKind::Variant(_, _, _, pats) => {
                for (i, p) in pats.iter().enumerate() {
                    self.bind(p, &v.field(i), by_ref)?;
                }
                self.consume(&v, by_ref, moves)?;
            }
            PatternKind::Struct(_, _, field_pats, _) => {
                for fp in field_pats {
                    let Value::Struct(_, fields) = self.memory.get(&v) else {
                        unsound("struct pattern on something else");
                    };
                    let i = fields.iter().position(|(f, _)| *f == fp.name.name);
                    let i = i.unwrap_or_else(|| unsound("no such field"));
                    self.bind(&fp.pat, &v.field(i), by_ref)?;
                }
                self.consume(&v, by_ref, moves)?;
            }
            PatternKind::Array(pats) => {
                for (i, p) in pats.iter().enumerate() {
                    self.bind(p, &v.field(i), by_ref)?;
                }
                self.consume(&v, by_ref, moves)?;
            }
            PatternKind::Or(alts) => {
                let alt = alts.iter().find(|alt| self.matches(alt, &v));
                let alt = alt.unwrap_or_else(|| unsound("no alternative matches"));
                self.bind(alt, &v, by_ref)?;
            }
            PatternKind::Move(_) => unsound("a pattern moving twice"),
            PatternKind::Wild
            | PatternKind::Int(..)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
        Ok(())
    }

    /// Uses up the value at `v`, which a pattern has just taken apart, if
    /// it `moves`, as the borrow checker's `Consume` does where it does
    /// (see `Builder::consume`): what the pattern's variables didn't move
    /// out of it is dropped, and nothing's left there. If it doesn't, the
    /// value must be `Discard`, if it's owned: the checker leaves only
    /// what it knew to be, which a type parameter is only if every type it
    /// can be is.
    fn consume(&mut self, v: &Ptr, by_ref: bool, moves: bool) -> Result<(), Error> {
        match (by_ref, moves, self.memory.get(v)) {
            (true, true, _) => unsound("using up what's behind a reference"),
            (false, true, _) => {
                let old = std::mem::replace(self.memory.get_mut(v), Value::Moved);
                return self.drop_parts(old);
            }
            (false, false, Value::Struct(ty, _) | Value::Variant(ty, ..))
                if !self.discard.contains(ty.as_str()) =>
            {
                unsound(&format!(
                    "a `{}`, which isn't `Discard`, taken apart and not used up",
                    crate::check::source_name(ty)
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// `==` on the values at `p` and `q`, at `span`. References and boxes
    /// compare what they point to. A struct or enum with an `eq` of its
    /// own, a function `<T as Eq>::eq` for its type `T`, compares by that;
    /// one with `#[derive(Eq)]` compares field by field, each by its own
    /// type's `==`.
    fn equal(&mut self, p: &Ptr, q: &Ptr, span: Span) -> Result<bool, Error> {
        // Slices, and strings, which are slices of bytes: of the same
        // length, with equal values at each index.
        if let (Some((a, n)), Some((b, m))) = (p.range, q.range) {
            if n != m {
                return Ok(false);
            }
            for i in 0..n {
                let (x, y) = (
                    Ptr {
                        range: None,
                        ..p.field(a + i)
                    },
                    Ptr {
                        range: None,
                        ..q.field(b + i)
                    },
                );
                if !self.equal(&x, &y, span)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        // The type, and how many fields to compare: none if they're
        // different variants.
        let (ty, fields) = match (self.memory.get(p), self.memory.get(q)) {
            (Value::Ref(p) | Value::Box(p), Value::Ref(q) | Value::Box(q)) => {
                let (p, q) = (p.clone(), q.clone());
                return self.equal(&p, &q, span);
            }
            (Value::Struct(s, xs), Value::Struct(..)) => (s.clone(), Some(xs.len())),
            (Value::Variant(s, v, xs), Value::Variant(_, w, _)) => {
                (s.clone(), (v == w).then_some(xs.len()))
            }
            // Arrays of a type, so of a length: value by value.
            (Value::Slots(xs), Value::Slots(_)) => (String::new(), Some(xs.len())),
            (Value::Moved, _) | (_, Value::Moved) => unsound("comparing a moved value"),
            (a, b) => return Ok(a == b),
        };
        if let Some(f) = self.user_eq(&ty).filter(|_| !ty.is_empty()) {
            return self.call_eq(f, p, q, span);
        }
        let Some(n) = fields else {
            return Ok(false);
        };
        for i in 0..n {
            if !self.equal(&p.field(i), &q.field(i), span)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The `eq` of a struct or enum's own, if type `ty` has one.
    ///
    /// Relies on monomorphisation having made a copy of every `eq` that a
    /// `==` can get to (`mono::Cx::need_eq`): here a missing one looks like
    /// a derived `Eq`, and the values would be compared field by field.
    fn user_eq(&self, ty: &str) -> Option<String> {
        let name = format!("<{ty} as {}>::eq", crate::traits::EQ);
        self.fns.contains_key(name.as_str()).then_some(name)
    }

    /// Calls `eq` `f` on what's at `p` and `q`.
    fn call_eq(&mut self, f: String, p: &Ptr, q: &Ptr, span: Span) -> Result<bool, Error> {
        let args = vec![Value::Ref(p.clone()), Value::Ref(q.clone())];
        Ok(bool(self.call(Value::Fn(f), args, span)?))
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
    fn arith(&mut self, op: BinOp, l: Value, r: Value, e: &Expr) -> Result<Value, Error> {
        let span = e.span;
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
                self.not_proved(e.id);
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
                self.not_proved(e.id);
                Err(self.panics(span, "integer overflow"))
            }
        }
    }
}

/// The root of place expression `e`: what its path of fields and `*`s
/// starts from, `v` in `(*v.f).g`. Any expression is a place expression,
/// its own root if it isn't a field or a `*`.
///
/// The borrow checker sees a place the same way (`Builder::place`), and the
/// two must agree on when each part of it is worked out: the root where
/// it's written, the path when the place is used. Only a borrow passed to a
/// call (`arguments`) tells those apart.
fn root(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Field(x, _) | ExprKind::Unary(UnOp::Deref, x) => root(x),
        _ => e,
    }
}
