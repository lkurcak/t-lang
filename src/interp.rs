//! A tree-walking interpreter. It gives the language its meaning (its
//! *dynamic semantics*) so we can check that the type system is right before
//! there's a real backend.
//!
//! Look at how often this file calls `unsound(..)`: every place where a value
//! could have the "wrong shape", like adding a bool, calling an int, or an `if`
//! on a number. The type checker promises those places are unreachable for
//! every program it accepts. That promise is the *type soundness* theorem
//! ("well-typed programs don't go wrong"), and those calls are where you'd see
//! it fail if the checker had a bug.

use crate::ast::*;
use crate::check::BUILTINS;
use crate::error::{Error, Span};
use std::collections::HashMap;
use std::fmt;

/// Deep enough for any reasonable recursion, shallow enough to stop runaway
/// recursion with an error instead of crashing the interpreter itself.
const MAX_CALL_DEPTH: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Unit,
    /// A top-level function, by name. No closures yet: a function can only see
    /// its own parameters and other top-level functions.
    Fn(String),
    Builtin(&'static str),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Unit => f.write_str("()"),
            Value::Fn(name) => write!(f, "<fn {name}>"),
            Value::Builtin(name) => write!(f, "<fn {name}>"),
        }
    }
}

/// Runs `main`. Anything the program prints is appended to `out`, even if the
/// program later fails.
pub fn run(prog: &Program, out: &mut Vec<String>) -> Result<Value, Error> {
    let fns = prog.fns.iter().map(|f| (f.name.name.as_str(), f)).collect();
    let mut interp = Interp { fns, locals: Vec::new(), depth: 0, out };
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
                let [arg] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("print_bool arity"));
                self.out.push(bool(arg).to_string());
                Ok(Value::Unit)
            }
            Value::Fn(name) => {
                let decl = self.fns[name.as_str()];
                if self.depth >= MAX_CALL_DEPTH {
                    return Err(Error::runtime(
                        span,
                        format!("stack overflow (more than {MAX_CALL_DEPTH} nested calls)"),
                    ));
                }
                // A call gets a fresh environment: just the parameters. The
                // caller's locals are saved and restored afterwards.
                let params = decl.params.iter().map(|p| p.name.name.clone());
                let saved = std::mem::replace(&mut self.locals, params.zip(args).collect());
                self.depth += 1;
                let result = self.eval(&decl.body);
                self.depth -= 1;
                self.locals = saved;
                result
            }
            other => unsound(&format!("called a non-function `{other}`")),
        }
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
                let args = args.iter().map(|a| self.eval(a)).collect::<Result<_, _>>()?;
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
                        Stmt::Let { name, init, .. } => {
                            let v = self.eval(init)?;
                            self.locals.push((name.name.clone(), v));
                        }
                        Stmt::Expr(e) => {
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
        })
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
