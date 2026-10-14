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
    /// A top-level function, by name. It can only see its own parameters and
    /// other top-level functions.
    Fn(String),
    Builtin(&'static str),
    /// A closure: an anonymous function, plus the local variables it can see.
    Closure(Closure),
    /// A struct: its type's name, and its fields in declaration order.
    Struct(String, Vec<(String, Value)>),
    /// An enum value: its type's name, which variant, and that variant's fields.
    Variant(String, String, Vec<Value>),
}

/// What `|x| x + n` evaluates to: its code, and the environment it was
/// created in, which holds `n`.
///
/// The environment is a *copy* of the locals the body uses, made at that
/// point. Since nothing in T can change a variable after it's bound, a copy
/// and the original can never disagree, so this is the same as sharing them.
/// It still has to live somewhere for as long as the closure does, which may
/// be longer than the function that made it (see docs/03-inference.md).
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
            Value::Fn(name) => write!(f, "<fn {name}>"),
            Value::Builtin(name) => write!(f, "<fn {name}>"),
            Value::Closure(_) => f.write_str("<closure>"),
            Value::Struct(name, fields) if fields.is_empty() => write!(f, "{name} {{}}"),
            Value::Struct(name, fields) => {
                write!(f, "{name} {{ ")?;
                for (i, (field, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{field}: {v}")?;
                }
                f.write_str(" }")
            }
            Value::Variant(ty, variant, fields) => {
                write!(f, "{ty}::{variant}")?;
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
        depth: 0,
        out,
    };
    interp.call(Value::Fn("main".to_string()), Vec::new(), Span::new(0, 0))
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

struct Interp<'p, 'o> {
    fns: HashMap<&'p str, &'p FnDecl>,
    /// For putting struct literals' fields in declaration order.
    structs: HashMap<&'p str, &'p StructDecl>,
    locals: Vec<(String, Value)>,
    depth: usize,
    out: &'o mut Vec<String>,
}

impl Interp<'_, '_> {
    fn lookup(&self, name: &str) -> Value {
        if let Some((_, v)) = self.locals.iter().rev().find(|(n, _)| n == name) {
            return v.clone();
        }
        if self.fns.contains_key(name) {
            return Value::Fn(name.to_string());
        }
        match BUILTINS.iter().find(|&&b| b == name) {
            Some(&b) => Value::Builtin(b),
            None => unsound(&format!("unbound variable `{name}`")),
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

    /// Evaluates a function body with `locals` as its environment. The
    /// caller's locals are saved and restored afterwards.
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
        let saved = std::mem::replace(&mut self.locals, locals);
        self.depth += 1;
        let result = self.eval(body);
        self.depth -= 1;
        self.locals = saved;
        result
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Error> {
        Ok(match &e.kind {
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Unit => Value::Unit,
            ExprKind::Var(name) => self.lookup(name),

            ExprKind::Unary(op, x) => {
                let v = self.eval(x)?;
                match op {
                    UnOp::Neg => Value::Int(
                        int(v)
                            .checked_neg()
                            .ok_or_else(|| Error::runtime(e.span, "integer overflow"))?,
                    ),
                    UnOp::Not => Value::Bool(!bool(v)),
                }
            }

            // `&&` and `||` short-circuit: the right side only runs if needed.
            ExprKind::Binary(BinOp::And, l, r) => {
                Value::Bool(bool(self.eval(l)?) && bool(self.eval(r)?))
            }
            ExprKind::Binary(BinOp::Or, l, r) => {
                Value::Bool(bool(self.eval(l)?) || bool(self.eval(r)?))
            }

            ExprKind::Binary(op, l, r) => {
                let l = self.eval(l)?;
                let r = self.eval(r)?;
                match op {
                    BinOp::Eq => Value::Bool(l == r),
                    BinOp::Ne => Value::Bool(l != r),
                    _ => self.arith(*op, int(l), int(r), e.span)?,
                }
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
                if bool(self.eval(c)?) {
                    self.eval(then)?
                } else if let Some(els) = els {
                    self.eval(els)?
                } else {
                    Value::Unit
                }
            }

            ExprKind::Block(stmts, tail) => {
                let scope = self.locals.len();
                for stmt in stmts {
                    match stmt {
                        Stmt::Let { pat, init, .. } => {
                            let v = self.eval(init)?;
                            if !self.bind(pat, &v) {
                                unsound(&format!("`let` pattern didn't match `{v}`"));
                            }
                        }
                        Stmt::Expr(e) | Stmt::BlockLike(e) => {
                            self.eval(e)?;
                        }
                    }
                }
                let v = match tail {
                    Some(tail) => self.eval(tail)?,
                    None => Value::Unit,
                };
                self.locals.truncate(scope);
                v
            }

            // These live in their own functions to keep `eval`'s stack frame
            // small: in debug builds every temporary in this `match` gets its
            // own slot, and deep recursion multiplies the frame size.
            ExprKind::StructLit(name, inits) => self.struct_lit(name, inits)?,
            ExprKind::Field(x, field) => self.field(x, field)?,
            ExprKind::Variant(ty, variant, args) => {
                let args = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<_, _>>()?;
                Value::Variant(ty.name.clone(), variant.name.clone(), args)
            }
            ExprKind::Match(scrutinee, arms) => self.match_expr(scrutinee, arms)?,
            // Only the locals the body uses: its free variables that aren't
            // top-level functions.
            ExprKind::Lambda(code) => Value::Closure(Closure {
                env: code
                    .captures
                    .iter()
                    .filter_map(|name| {
                        let local = self.locals.iter().rev().find(|(n, _)| n == name);
                        local.cloned()
                    })
                    .collect(),
                code: code.clone(),
            }),
        })
    }

    fn struct_lit(&mut self, name: &Ident, inits: &[FieldInit]) -> Result<Value, Error> {
        let decl = self.structs[name.name.as_str()];
        // Evaluate in the order written, store in the order declared.
        let mut values = Vec::new();
        for init in inits {
            values.push((init.name.name.as_str(), self.eval(&init.value)?));
        }
        let fields = decl
            .fields
            .iter()
            .map(|f| {
                let i = values.iter().position(|(n, _)| *n == f.name.name);
                let i = i.unwrap_or_else(|| unsound("missing struct field"));
                (f.name.name.clone(), values[i].1.clone())
            })
            .collect();
        Ok(Value::Struct(name.name.clone(), fields))
    }

    fn field(&mut self, x: &Expr, field: &Ident) -> Result<Value, Error> {
        match self.eval(x)? {
            Value::Struct(_, fields) => match fields.into_iter().find(|(f, _)| *f == field.name) {
                Some((_, v)) => Ok(v),
                None => unsound(&format!("no field `{}`", field.name)),
            },
            other => unsound(&format!("field access on `{other}`")),
        }
    }

    fn match_expr(&mut self, scrutinee: &Expr, arms: &[Arm]) -> Result<Value, Error> {
        let v = self.eval(scrutinee)?;
        let scope = self.locals.len();
        let mut chosen = None;
        for arm in arms {
            if self.bind(&arm.pat, &v) {
                chosen = Some(arm);
                break;
            }
            // Drop whatever a partial match bound before failing.
            self.locals.truncate(scope);
        }
        let Some(arm) = chosen else {
            unsound(&format!("no `match` arm fits `{v}`"));
        };
        let result = self.eval(&arm.body);
        self.locals.truncate(scope);
        result
    }

    /// Tries to match `v` against `p`. On success, pushes the variables `p`
    /// binds onto `locals` and returns `true`.
    fn bind(&mut self, p: &Pattern, v: &Value) -> bool {
        match (&p.kind, v) {
            (PatternKind::Wild, _) => true,
            (PatternKind::Bind(name), _) => {
                self.locals.push((name.clone(), v.clone()));
                true
            }
            (PatternKind::Int(n), Value::Int(m)) => n == m,
            (PatternKind::Range(lo, hi, _), Value::Int(m)) => lo <= m && m <= hi,
            (PatternKind::Bool(b), Value::Bool(c)) => b == c,
            (PatternKind::Unit, Value::Unit) => true,
            (PatternKind::Variant(_, variant, pats), Value::Variant(_, name, fields)) => {
                variant.name == *name && pats.iter().zip(fields).all(|(p, v)| self.bind(p, v))
            }
            (PatternKind::Or(alts), _) => {
                let scope = self.locals.len();
                alts.iter().any(|alt| {
                    // Undo anything a failed alternative bound part-way.
                    self.locals.truncate(scope);
                    self.bind(alt, v)
                })
            }
            (PatternKind::Struct(_, field_pats, _), Value::Struct(_, fields)) => field_pats
                .iter()
                .all(|fp| match fields.iter().find(|(f, _)| *f == fp.name.name) {
                    Some((_, v)) => self.bind(&fp.pat, v),
                    None => unsound(&format!("no field `{}`", fp.name.name)),
                }),
            _ => unsound(&format!("pattern can't apply to `{v}`")),
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
