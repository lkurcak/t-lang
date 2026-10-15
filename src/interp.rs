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

use crate::ast::*;
use crate::check::BUILTINS;
use crate::error::{Error, Span};
use std::collections::HashMap;
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

impl Value {
    /// Does this value own a box, so that copying it would mean two owners?
    /// Then using it moves it. Through a reference doesn't count: that box
    /// is someone else's. Everything else is copied: whether that's allowed
    /// is the checker's business, not memory's.
    fn owns_box(&self) -> bool {
        match self {
            Value::Box(_) => true,
            Value::Struct(_, fields) => fields.iter().any(|(_, v)| v.owns_box()),
            Value::Variant(_, _, fields) => fields.iter().any(Value::owns_box),
            Value::Moved => unsound("use of a value that was moved out of, at least in part"),
            _ => false,
        }
    }
}

/// Runs `main`. Anything the program prints is appended to `out`, even if the
/// program later fails.
pub fn run(prog: &Program, out: &mut Vec<String>) -> Result<Value, Error> {
    let fns = prog.fns.iter().map(|f| (f.name.name.as_str(), f)).collect();
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

    /// The value at `p`, moved out if it owns a box, or else copied.
    fn read(&mut self, p: &Ptr) -> Value {
        let v = self.memory.get(p);
        if let Value::Moved = v {
            unsound("use of a value that was moved");
        }
        if v.owns_box() {
            std::mem::replace(self.memory.get_mut(p), Value::Moved)
        } else {
            v.clone()
        }
    }

    /// Frees everything `v` owns: the boxes in it, and what's in those.
    fn drop(&mut self, v: Value) {
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
    fn statement<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error> {
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

    /// Where the value of place expression `e` is. Anything else is
    /// evaluated into a temporary, which is where it is.
    fn place(&mut self, e: &Expr) -> Result<Ptr, Error> {
        match &e.kind {
            ExprKind::Var(name, _) => match self.local(name) {
                Some(p) => Ok(p),
                None => {
                    let f = self.function(name);
                    Ok(self.temp(f))
                }
            },
            // Through any references and boxes to the struct.
            ExprKind::Field(x, field) => {
                let mut p = self.place(x)?;
                loop {
                    match self.memory.get(&p) {
                        Value::Ref(q) | Value::Box(q) => p = q.clone(),
                        Value::Struct(_, fields) => {
                            let i = fields.iter().position(|(f, _)| *f == field.name);
                            let i =
                                i.unwrap_or_else(|| unsound(&format!("no field `{}`", field.name)));
                            return Ok(p.field(i));
                        }
                        other => unsound(&format!("field access on `{other}`")),
                    }
                }
            }
            ExprKind::Unary(UnOp::Deref, x) => {
                let p = self.place(x)?;
                match self.memory.get(&p) {
                    Value::Ref(q) | Value::Box(q) => Ok(q.clone()),
                    other => unsound(&format!("`*` on `{other}`")),
                }
            }
            _ => {
                let v = self.eval(e)?;
                Ok(self.temp(v))
            }
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
            return Err(Error::runtime(
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
        result
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Error> {
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
                    UnOp::Neg => Value::Int(
                        int(v)
                            .checked_neg()
                            .ok_or_else(|| Error::runtime(e.span, "integer overflow"))?,
                    ),
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
                let same = self.equal(self.memory.get(&l), self.memory.get(&r));
                Value::Bool(same == (*op == BinOp::Eq))
            }

            ExprKind::Binary(op, l, r) => {
                let l = self.eval(l)?;
                let r = self.eval(r)?;
                self.arith(*op, int(l), int(r), e.span)?
            }

            ExprKind::Call(f, args) => {
                let f = self.eval(f)?;
                let args = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<_, _>>()?;
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

            ExprKind::While(c, body) => {
                while bool(self.statement(|i| i.eval(c))?) {
                    self.eval(body)?;
                }
                Value::Unit
            }

            // The value first, then the place, as in Rust. What was there
            // is dropped.
            ExprKind::Assign(op, place, value) => {
                let v = self.eval(value)?;
                let p = self.place(place)?;
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
            ExprKind::Variant(ty, _, _, args) if ty.name == "Box" => {
                let v = self.eval(&args[0])?;
                Value::Box(self.memory.alloc(v, true))
            }
            ExprKind::Variant(ty, _, variant, args) => {
                let args = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<_, _>>()?;
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
    fn block(&mut self, stmts: &[Stmt], tail: Option<&Expr>) -> Result<Value, Error> {
        let scope = self.locals.len();
        let result = self.block_inner(stmts, tail);
        self.end_scope(scope);
        result
    }

    fn block_inner(&mut self, stmts: &[Stmt], tail: Option<&Expr>) -> Result<Value, Error> {
        for stmt in stmts {
            match stmt {
                Stmt::Let { pat, init, .. } => self.let_stmt(pat, init)?,
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
    fn let_stmt(&mut self, pat: &Pattern, init: &Expr) -> Result<(), Error> {
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

    fn struct_lit(&mut self, name: &Ident, inits: &[FieldInit]) -> Result<Value, Error> {
        let decl = self.structs[name.name.as_str()];
        // Evaluate in the order written, store in the order declared.
        let mut values = Vec::new();
        for init in inits {
            values.push((init.name.name.as_str(), Some(self.eval(&init.value)?)));
        }
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
    fn match_expr(&mut self, scrutinee: &Expr, arms: &[Arm]) -> Result<Value, Error> {
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

    /// Are `a` and `b` equal, looking through references and boxes to what
    /// they point to?
    fn equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Ref(p) | Value::Box(p), Value::Ref(q) | Value::Box(q)) => {
                self.equal(self.memory.get(p), self.memory.get(q))
            }
            (Value::Struct(s, xs), Value::Struct(t, ys)) => {
                s == t && xs.iter().zip(ys).all(|((_, x), (_, y))| self.equal(x, y))
            }
            (Value::Variant(s, v, xs), Value::Variant(t, w, ys)) => {
                s == t && v == w && xs.iter().zip(ys).all(|(x, y)| self.equal(x, y))
            }
            (Value::Moved, _) | (_, Value::Moved) => unsound("comparing a moved value"),
            (a, b) => a == b,
        }
    }

    fn arith(&self, op: BinOp, l: i64, r: i64, span: Span) -> Result<Value, Error> {
        let overflow = || Error::runtime(span, "integer overflow");
        Ok(match op {
            BinOp::Add => Value::Int(l.checked_add(r).ok_or_else(overflow)?),
            BinOp::Sub => Value::Int(l.checked_sub(r).ok_or_else(overflow)?),
            BinOp::Mul => Value::Int(l.checked_mul(r).ok_or_else(overflow)?),
            BinOp::Div | BinOp::Rem if r == 0 => {
                return Err(Error::runtime(span, "division by zero"));
            }
            BinOp::Div => Value::Int(l.checked_div(r).ok_or_else(overflow)?),
            BinOp::Rem => Value::Int(l.checked_rem(r).ok_or_else(overflow)?),
            BinOp::Lt => Value::Bool(l < r),
            BinOp::Le => Value::Bool(l <= r),
            BinOp::Gt => Value::Bool(l > r),
            BinOp::Ge => Value::Bool(l >= r),
            BinOp::Eq | BinOp::Ne | BinOp::And | BinOp::Or => unreachable!("handled in eval"),
        })
    }
}
