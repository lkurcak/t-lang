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
//! Since step 6a.1 it keeps linear values the same way, by their types:
//! what monomorphisation made says which are `Copy` and which `Discard`.
//! Using a value whose type isn't `Copy` moves it, whatever it holds, and a
//! pattern that takes apart a value whose type isn't `Discard`, by value,
//! uses it up, as the borrow checker's `Consume` does. Throwing one away
//! whole, rather than using it up, is a leak too
//! (docs/06a.1-linear-types.md). Except on the way out of a run-time error:
//! that's a panic, which aborts, so nothing is really dropped.
//!
//! `return`, `break` and `continue` unwind: each is a `Flow` that runs back
//! up through the expressions it's in, ending their statements and scopes
//! on the way, to the call or loop it goes to. What those expressions had
//! worked out so far is dropped, as a compiled program would: in
//! `f(Box::new(1), return 2)`, the box.

use crate::ast::*;
use crate::check::BUILTINS;
use crate::error::{Error, Span};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// Deep enough for any reasonable recursion, shallow enough to stop runaway
/// recursion with an error instead of crashing the interpreter itself.
const MAX_CALL_DEPTH: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Unit,
    /// A function, by name. It can only see its own parameters and other
    /// functions. Nested ones have been lifted to the top level by now,
    /// under their symbols (see `lift.rs`).
    Fn(String),
    Builtin(&'static str),
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
}

impl Ptr {
    /// Field `i` of the value this points to.
    fn field(&self, i: usize) -> Ptr {
        let mut p = self.clone();
        p.path.push(i);
        p
    }
}

/// What `|x| x + n` evaluates to: its code, and the environment it was
/// created in, which holds `n`.
///
/// The environment is a *copy* of the locals the body uses, made at that
/// point. What a closure can capture is `Copy` and holds no references, so
/// a copy is as good as the original. It still has to live somewhere for as
/// long as the closure does, which may be longer than the function that made
/// it (see docs/03-inference.md).
///
/// Copies of the closure share one environment, so copying a closure that
/// holds closures that hold closures costs no more than copying one.
#[derive(Debug, Clone)]
pub struct Closure {
    pub code: Arc<Lambda>,
    pub env: Arc<[(String, Value)]>,
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
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Unit => f.write_str("()"),
            Value::Fn(name) => write!(f, "<fn {}>", crate::check::source_name(name)),
            Value::Builtin(name) => write!(f, "<fn {name}>"),
            Value::Closure(_) => f.write_str("<closure>"),
            Value::Ref(_) => f.write_str("<reference>"),
            Value::Box(_) => f.write_str("<box>"),
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

/// Runs `main`. Anything the program prints is appended to `out`, even if the
/// program later fails.
pub fn run(prog: &Program, out: &mut Vec<String>) -> Result<Value, Error> {
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
            TypeDecl::Enum(_) => None,
        })
        .collect();
    let mut interp = Interp {
        fns,
        structs,
        copy: derived("Copy"),
        discard: derived("Discard"),
        aborting: false,
        locals: Vec::new(),
        memory: Memory::default(),
        temps: Vec::new(),
        depth: 0,
        out,
    };
    let result = interp.call(Value::Fn("main".to_string()), Vec::new(), Span::new(0, 0))?;
    interp.drop(result.clone());
    if interp.memory.on_heap > 0 {
        unsound(&format!(
            "memory leak: {} boxes were never freed",
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

fn int(v: Value) -> i64 {
    match v {
        Value::Int(n) => n,
        other => unsound(&format!("expected an int, got `{other}`")),
    }
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
    locals: Vec<(String, Ptr)>,
    memory: Memory,
    /// The temporaries made in each statement being run, innermost last,
    /// to free when it's done.
    temps: Vec<Vec<Ptr>>,
    depth: usize,
    out: &'o mut Vec<String>,
}

impl Interp<'_, '_> {
    /// The slot of local variable `name`, if it's one.
    fn local(&self, name: &str) -> Option<Ptr> {
        let local = self.locals.iter().rev().find(|(n, _)| n == name);
        local.map(|(_, p)| p.clone())
    }

    /// The function `name` names, which isn't a local.
    fn function(&self, name: &str) -> Value {
        if self.fns.contains_key(name) {
            return Value::Fn(name.to_string());
        }
        match BUILTINS.iter().find(|&&b| b == name) {
            Some(&b) => Value::Builtin(b),
            None => unsound(&format!("unbound variable `{name}`")),
        }
    }

    /// The value at `p`, moved out if its type isn't `Copy`, or else
    /// copied. It must all be there.
    fn read(&mut self, p: &Ptr) -> Value {
        let v = self.memory.get(p);
        if self.taken_apart(v) {
            unsound("use of a value that was moved out of, at least in part");
        }
        if self.moves(v) {
            std::mem::replace(self.memory.get_mut(p), Value::Moved)
        } else {
            v.clone()
        }
    }

    /// Is using `v` moving it: is its type not `Copy`? A box never is, and
    /// a reference always is, as far as memory goes: that a `&mut` isn't is
    /// the borrow checker's business.
    fn moves(&self, v: &Value) -> bool {
        match v {
            Value::Box(_) => true,
            Value::Struct(ty, _) | Value::Variant(ty, ..) => !self.copy.contains(ty.as_str()),
            _ => false,
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
            _ => false,
        }
    }

    /// Frees everything `v` owns: the boxes in it, and what's in those.
    ///
    /// Throwing away a struct or enum value that isn't `Discard` is a leak,
    /// unless it's been taken apart, by moving out of it: then only what's
    /// left in it counts, each part on its own, as for the borrow checker
    /// (`moves::leftover`).
    fn drop(&mut self, v: Value) {
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
        self.drop_parts(v);
    }

    /// Drops what `v` holds, but not `v` itself.
    fn drop_parts(&mut self, v: Value) {
        match v {
            Value::Box(p) => {
                let inner = self.memory.free(&p, true);
                self.drop(inner);
            }
            Value::Struct(_, fields) => fields.into_iter().for_each(|(_, v)| self.drop(v)),
            Value::Variant(_, _, fields) => fields.into_iter().for_each(|v| self.drop(v)),
            _ => {}
        }
    }

    /// The error at `span`, of a program that panics: from here on,
    /// nothing is dropped for real.
    fn panic(&mut self, span: Span, message: impl Into<String>) -> Error {
        self.aborting = true;
        Error::runtime(span, message)
    }

    /// Drops the variables from `scope` on, the last declared first, and
    /// frees their slots.
    fn end_scope(&mut self, scope: usize) {
        while self.locals.len() > scope {
            let (_, p) = self.locals.pop().expect("a local");
            let v = self.memory.free(&p, false);
            self.drop(v);
        }
    }

    /// Runs `f` as a statement: the temporaries it makes are dropped, in
    /// reverse, when it's done.
    fn statement<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, Flow>) -> Result<T, Flow> {
        self.temps.push(Vec::new());
        let result = f(self);
        for p in self.temps.pop().expect("pushed").into_iter().rev() {
            let v = self.memory.free(&p, false);
            self.drop(v);
        }
        result
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
                        Value::Struct(_, fields) => {
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
            Value::Builtin("print") => {
                let [arg] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("print arity"));
                self.out.push(int(arg).to_string());
                Ok(Value::Unit)
            }
            Value::Builtin("print_bool") => {
                let [arg] =
                    <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("print_bool arity"));
                self.out.push(bool(arg).to_string());
                Ok(Value::Unit)
            }
            // What was behind the reference comes out, and the new value
            // goes in: nothing is dropped.
            Value::Builtin("replace") => {
                let [r, new] =
                    <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("replace arity"));
                let Value::Ref(p) = r else {
                    unsound(&format!("`replace` of `{r}`"))
                };
                let old = std::mem::replace(self.memory.get_mut(&p), new);
                if let Value::Moved = old {
                    unsound("`replace` of a value that was moved");
                }
                Ok(old)
            }
            Value::Builtin("swap") => {
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
            Value::Fn(name) => {
                let decl = self.fns[name.as_str()];
                // A call gets a fresh environment: just the parameters.
                let params = decl.params.iter().map(|p| p.name.name.clone());
                self.enter(params.zip(args).collect(), &decl.body, span)
            }
            // A closure's environment is the one it was created in, plus its
            // parameters.
            Value::Closure(Closure { code, env }) => {
                let params = code.params.iter().map(|p| p.name.name.clone());
                let locals = env.iter().cloned().chain(params.zip(args)).collect();
                self.enter(locals, &code.body, span)
            }
            other => unsound(&format!("called a non-function `{other}`")),
        }
    }

    /// Evaluates a function body with `locals` as its environment, each in
    /// a new slot, freed at the end. The caller's locals are saved and
    /// restored afterwards.
    fn enter(
        &mut self,
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
        let locals = locals
            .into_iter()
            .map(|(name, v)| (name, self.memory.alloc(v, false)))
            .collect();
        let saved = std::mem::replace(&mut self.locals, locals);
        let saved_temps = std::mem::take(&mut self.temps);
        self.depth += 1;
        let result = self.statement(|i| i.eval(body));
        self.depth -= 1;
        self.end_scope(0);
        self.locals = saved;
        self.temps = saved_temps;
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
                    values.into_iter().rev().for_each(|v| self.drop(v));
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
    fn arguments(&mut self, args: &[Expr]) -> Result<Vec<Value>, Flow> {
        // Each argument's value, or for a borrow, where its place's root is.
        let mut firsts: Vec<Result<Value, Ptr>> = Vec::new();
        for a in args {
            let first = match &a.kind {
                ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) => self.base(root(x)).map(Err),
                _ => self.eval(a).map(Ok),
            };
            match first {
                Ok(first) => firsts.push(first),
                Err(flow) => {
                    for v in firsts.into_iter().rev().filter_map(Result::ok) {
                        self.drop(v);
                    }
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
                Ok(v) => self.drop(v),
                Err(Flow::Break(l, v)) if ours(&l) => return Ok(v),
                Err(Flow::Continue(l)) if ours(&l) => {}
                Err(flow) => return Err(flow),
            }
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Flow> {
        Ok(match &e.kind {
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Unit => Value::Unit,
            ExprKind::Var(name, _) => match self.local(name) {
                Some(p) => self.read(&p),
                None => self.function(name),
            },
            ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _) => {
                let p = self.place(e)?;
                self.read(&p)
            }
            ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) => Value::Ref(self.place(x)?),

            ExprKind::Unary(op, x) => {
                let v = self.eval(x)?;
                match op {
                    UnOp::Neg => match int(v).checked_neg() {
                        Some(n) => Value::Int(n),
                        None => return Err(self.panic(e.span, "integer overflow").into()),
                    },
                    _ => Value::Bool(!bool(v)),
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
                self.arith(*op, int(l), int(r), e.span)?
            }

            // A function value owns nothing, so there's nothing to drop if
            // an argument doesn't finish.
            ExprKind::Call(f, args) => {
                let f = self.eval(f)?;
                let args = self.arguments(args)?;
                self.call(f, args, e.span)?
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
                        self.drop(v);
                        return Err(flow);
                    }
                };
                let v = match op {
                    Some(op) => {
                        let old = int(self.memory.get(&p).clone());
                        self.arith(*op, old, int(v), e.span)?
                    }
                    None => v,
                };
                let old = std::mem::replace(self.memory.get_mut(&p), v);
                self.drop(old);
                Value::Unit
            }

            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref())?,

            // These live in their own functions to keep `eval`'s stack frame
            // small: in debug builds every temporary in this `match` gets its
            // own slot, and deep recursion multiplies the frame size.
            ExprKind::StructLit(name, _, inits) => self.struct_lit(name, inits)?,
            ExprKind::MethodCall(..) => unreachable!("lifting makes method calls plain calls"),
            ExprKind::Variant(ty, _, _, args, ..) if ty.name == "Box" => {
                let v = self.eval(&args[0])?;
                Value::Box(self.memory.alloc(v, true))
            }
            ExprKind::Variant(ty, _, variant, args, ..) => {
                let args = self.eval_all(args)?;
                Value::Variant(ty.name.clone(), variant.name.clone(), args)
            }
            ExprKind::Match(scrutinee, arms) => self.match_expr(scrutinee, arms)?,
            // Only the locals the body uses: its free variables that aren't
            // functions. They're `Copy`, so they're copied.
            ExprKind::Lambda(code) => {
                let mut env = Vec::new();
                for name in &code.captures {
                    if let Some(p) = self.local(name) {
                        let v = self.read(&p);
                        env.push((name.clone(), v));
                    }
                }
                Value::Closure(Closure {
                    env: env.into(),
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
        self.end_scope(scope);
        result
    }

    fn block_inner(&mut self, stmts: &[Stmt], tail: Option<&Expr>) -> Result<Value, Flow> {
        for stmt in stmts {
            match stmt {
                Stmt::Let {
                    pat,
                    init: Some(init),
                    ..
                } => self.let_stmt(pat, init)?,
                // No value yet: as if it had been moved out of.
                Stmt::Let {
                    pat, init: None, ..
                } => {
                    let PatternKind::Bind(name, _) = &pat.kind else {
                        unsound("`let` with no value of a pattern");
                    };
                    let slot = self.memory.alloc(Value::Moved, false);
                    self.locals.push((name.clone(), slot));
                }
                Stmt::Expr(e) | Stmt::BlockLike(e) => {
                    self.statement(|i| {
                        let v = i.eval(e)?;
                        i.drop(v);
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

    /// `let pat = init;`. If `init` is `&e` of a value that isn't a place,
    /// the temporary holding it lives as long as a variable would, to the
    /// end of the block, as in Rust: `let r = &make();` is fine.
    fn let_stmt(&mut self, pat: &Pattern, init: &Expr) -> Result<(), Flow> {
        if let ExprKind::Unary(UnOp::Ref | UnOp::RefMut, x) = &init.kind
            && !x.is_place()
        {
            let v = self.statement(|i| i.eval(x))?;
            let p = self.memory.alloc(v, false);
            self.locals.push((String::new(), p.clone()));
            let r = self.memory.alloc(Value::Ref(p), false);
            self.temps.push(vec![r.clone()]);
            self.bind(pat, &r, false);
            let temps = self.temps.pop().expect("pushed");
            for p in temps {
                self.memory.free(&p, false);
            }
            return Ok(());
        }
        self.statement(|i| {
            let p = i.place(init)?;
            if !i.matches(pat, &p) {
                unsound("a `let` pattern didn't match");
            }
            i.bind(pat, &p, false);
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
        Ok(Value::Struct(name.name.clone(), fields))
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
            i.bind(&arm.pat, &p, false);
            let result = i.eval(&arm.body);
            i.end_scope(scope);
            result
        })
    }

    /// Does `p` match the value at `v`? Patterns that look inside a value
    /// look through references to it, as the checker's do.
    fn matches(&self, p: &Pattern, v: &Ptr) -> bool {
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
            (PatternKind::Int(n), Value::Int(m)) => n == m,
            (PatternKind::Range(lo, hi, _), Value::Int(m)) => lo <= m && m <= hi,
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
    /// what it matches, moved or copied out, or a reference to it if
    /// `by_ref` or the pattern looked through a reference to get there.
    fn bind(&mut self, p: &Pattern, v: &Ptr, mut by_ref: bool) {
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
                let value = match by_ref {
                    true => Value::Ref(v),
                    false => self.read(&v),
                };
                let slot = self.memory.alloc(value, false);
                self.locals.push((name.clone(), slot));
            }
            PatternKind::Variant(_, _, _, pats) => {
                for (i, p) in pats.iter().enumerate() {
                    self.bind(p, &v.field(i), by_ref);
                }
                self.consume(&v, by_ref);
            }
            PatternKind::Struct(_, _, field_pats, _) => {
                for fp in field_pats {
                    let Value::Struct(_, fields) = self.memory.get(&v) else {
                        unsound("struct pattern on something else");
                    };
                    let i = fields.iter().position(|(f, _)| *f == fp.name.name);
                    let i = i.unwrap_or_else(|| unsound("no such field"));
                    self.bind(&fp.pat, &v.field(i), by_ref);
                }
                self.consume(&v, by_ref);
            }
            PatternKind::Or(alts) => {
                let alt = alts.iter().find(|alt| self.matches(alt, &v));
                let alt = alt.unwrap_or_else(|| unsound("no alternative matches"));
                self.bind(alt, &v, by_ref);
            }
            PatternKind::Wild
            | PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
    }

    /// Uses up the value at `v`, which a pattern has just taken apart, if
    /// it's owned, not reached `by_ref`, and isn't `Discard`: what the
    /// pattern's variables didn't move out of it is dropped, and nothing's
    /// left there. As the borrow checker's `Consume` does, though that goes
    /// by the type the checker knows (see `Builder::consume`).
    fn consume(&mut self, v: &Ptr, by_ref: bool) {
        if let Value::Struct(ty, _) | Value::Variant(ty, ..) = self.memory.get(v)
            && !by_ref
            && !self.discard.contains(ty.as_str())
        {
            let old = std::mem::replace(self.memory.get_mut(v), Value::Moved);
            self.drop_parts(old);
        }
    }

    /// `==` on the values at `p` and `q`, at `span`. References and boxes
    /// compare what they point to. A struct or enum with an `eq` of its
    /// own, a function `<T as Eq>::eq` for its type `T`, compares by that;
    /// one with `#[derive(Eq)]` compares field by field, each by its own
    /// type's `==`.
    fn equal(&mut self, p: &Ptr, q: &Ptr, span: Span) -> Result<bool, Error> {
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
            (Value::Moved, _) | (_, Value::Moved) => unsound("comparing a moved value"),
            (a, b) => return Ok(a == b),
        };
        if let Some(f) = self.user_eq(&ty) {
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
        let name = format!("<{ty} as Eq>::eq");
        self.fns.contains_key(name.as_str()).then_some(name)
    }

    /// Calls `eq` `f` on what's at `p` and `q`.
    fn call_eq(&mut self, f: String, p: &Ptr, q: &Ptr, span: Span) -> Result<bool, Error> {
        let args = vec![Value::Ref(p.clone()), Value::Ref(q.clone())];
        Ok(bool(self.call(Value::Fn(f), args, span)?))
    }

    fn arith(&mut self, op: BinOp, l: i64, r: i64, span: Span) -> Result<Value, Error> {
        let n = match op {
            BinOp::Lt => return Ok(Value::Bool(l < r)),
            BinOp::Le => return Ok(Value::Bool(l <= r)),
            BinOp::Gt => return Ok(Value::Bool(l > r)),
            BinOp::Ge => return Ok(Value::Bool(l >= r)),
            BinOp::Div | BinOp::Rem if r == 0 => {
                return Err(self.panic(span, "division by zero"));
            }
            BinOp::Add => l.checked_add(r),
            BinOp::Sub => l.checked_sub(r),
            BinOp::Mul => l.checked_mul(r),
            BinOp::Div => l.checked_div(r),
            BinOp::Rem => l.checked_rem(r),
            BinOp::Eq | BinOp::Ne | BinOp::And | BinOp::Or => unreachable!("handled in eval"),
        };
        match n {
            Some(n) => Ok(Value::Int(n)),
            None => Err(self.panic(span, "integer overflow")),
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
