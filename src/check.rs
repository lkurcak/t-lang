//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `int`, `bool`, `()` and function types, plus `let`, `if` and
//! top-level recursive functions.
//!
//! Each case below is an implementation of a *typing rule*, written above it in
//! the usual inference-rule notation:
//!
//! ```text
//!   premise 1    premise 2
//!   ---------------------- (Rule-Name)
//!        conclusion
//! ```
//!
//! read as "if the premises hold, the conclusion holds". The judgment
//! `Γ ⊢ e : T` reads "in context Γ, expression e has type T", where the
//! context Γ maps variable names to their types (here: `locals` + `globals`).
//!
//! Every function here is *syntax-directed*: for each form of expression
//! exactly one rule applies, so the rules can be read directly as a recursive
//! algorithm. That's not true of every type system (see docs/01-stlc.md).

use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::Type;
use std::collections::HashMap;

/// Built-in functions, available everywhere. Shared with the interpreter.
pub const BUILTINS: &[&str] = &["print", "print_bool"];

fn builtin_type(name: &str) -> Type {
    match name {
        // print : fn(int) -> ()
        "print" => Type::Fn(vec![Type::Int], Box::new(Type::Unit)),
        // print_bool : fn(bool) -> ()
        "print_bool" => Type::Fn(vec![Type::Bool], Box::new(Type::Unit)),
        _ => unreachable!("unknown builtin `{name}`"),
    }
}

pub fn check(prog: &Program) -> Result<(), Error> {
    let mut globals = HashMap::new();
    for &b in BUILTINS {
        globals.insert(b.to_string(), builtin_type(b));
    }

    // Pass 1: collect every function's signature first, so bodies can refer to
    // functions defined later in the file (and to themselves: recursion).
    for f in &prog.fns {
        let params = f.params.iter().map(|p| resolve(&p.ty)).collect::<Result<Vec<_>, _>>()?;
        let ret = resolve(&f.ret)?;
        let ty = Type::Fn(params, Box::new(ret));
        if globals.insert(f.name.name.clone(), ty).is_some() {
            return Err(Error::ty(f.name.span, format!("`{}` is already defined", f.name.name)));
        }
    }

    // Pass 2: check each body against its declared return type.
    let mut cx = Checker { globals, locals: Vec::new() };
    for f in &prog.fns {
        cx.locals.clear();
        for (i, p) in f.params.iter().enumerate() {
            if f.params[..i].iter().any(|q| q.name.name == p.name.name) {
                return Err(Error::ty(
                    p.name.span,
                    format!("parameter `{}` is declared twice", p.name.name),
                ));
            }
            cx.locals.push((p.name.name.clone(), resolve(&p.ty)?));
        }
        cx.check(&f.body, &resolve(&f.ret)?)?;
    }

    match prog.fns.iter().find(|f| f.name.name == "main") {
        None => Err(Error::ty(Span::new(0, 0), "program has no `main` function")),
        Some(main) if !main.params.is_empty() => {
            Err(Error::ty(main.name.span, "`main` must not take parameters"))
        }
        Some(_) => Ok(()),
    }
}

/// Turns a type as written (`TypeExpr`) into a type as understood (`Type`).
fn resolve(t: &TypeExpr) -> Result<Type, Error> {
    match &t.kind {
        TypeExprKind::Named(name) => match name.as_str() {
            "int" => Ok(Type::Int),
            "bool" => Ok(Type::Bool),
            _ => Err(Error::ty(t.span, format!("unknown type `{name}`"))),
        },
        TypeExprKind::Unit => Ok(Type::Unit),
        TypeExprKind::Fn(params, ret) => Ok(Type::Fn(
            params.iter().map(resolve).collect::<Result<_, _>>()?,
            Box::new(resolve(ret)?),
        )),
    }
}

struct Checker {
    globals: HashMap<String, Type>,
    /// The local part of the context Γ, as a stack. Entering a scope pushes,
    /// leaving truncates. Searching from the top makes inner `let`s shadow
    /// outer ones.
    locals: Vec<(String, Type)>,
}

impl Checker {
    fn lookup(&self, name: &str) -> Option<Type> {
        self.locals
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
            .or_else(|| self.globals.get(name).cloned())
    }

    /// Checks that `e` has type `want`.
    ///
    /// In STLC every expression's type can be computed bottom-up, so checking
    /// is just "synthesize, then compare". In step 3 (bidirectional typing)
    /// checking becomes its own mode that can push type information *down*
    /// into an expression.
    fn check(&mut self, e: &Expr, want: &Type) -> Result<(), Error> {
        let got = self.synth(e)?;
        if got == *want {
            Ok(())
        } else {
            Err(Error::ty(result_span(e), format!("expected `{want}`, found `{got}`")))
        }
    }

    /// Computes ("synthesizes") the type of `e`.
    fn synth(&mut self, e: &Expr) -> Result<Type, Error> {
        match &e.kind {
            //  ----------------- (T-Int)       ------------------- (T-True/False)
            //   Γ ⊢ 42 : int                    Γ ⊢ true : bool
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Bool(_) => Ok(Type::Bool),

            //  -------------- (T-Unit)
            //   Γ ⊢ () : ()
            ExprKind::Unit => Ok(Type::Unit),

            //   (x : T) ∈ Γ
            //  ------------- (T-Var)
            //   Γ ⊢ x : T
            ExprKind::Var(name) => self
                .lookup(name)
                .ok_or_else(|| Error::ty(e.span, format!("unknown variable `{name}`"))),

            //   Γ ⊢ e : int            Γ ⊢ e : bool
            //  -------------- (T-Neg)  -------------- (T-Not)
            //   Γ ⊢ -e : int            Γ ⊢ !e : bool
            ExprKind::Unary(op, x) => {
                let t = match op {
                    UnOp::Neg => Type::Int,
                    UnOp::Not => Type::Bool,
                };
                self.check(x, &t)?;
                Ok(t)
            }

            ExprKind::Binary(op, l, r) => match op {
                //   Γ ⊢ l : int   Γ ⊢ r : int
                //  --------------------------- (T-Arith)   for + - * / %
                //       Γ ⊢ l + r : int
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                    self.check(l, &Type::Int)?;
                    self.check(r, &Type::Int)?;
                    Ok(Type::Int)
                }

                //   Γ ⊢ l : int   Γ ⊢ r : int
                //  --------------------------- (T-Cmp)     for < <= > >=
                //       Γ ⊢ l < r : bool
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    self.check(l, &Type::Int)?;
                    self.check(r, &Type::Int)?;
                    Ok(Type::Bool)
                }

                //   Γ ⊢ l : bool   Γ ⊢ r : bool
                //  ----------------------------- (T-Logic) for && ||
                //       Γ ⊢ l && r : bool
                BinOp::And | BinOp::Or => {
                    self.check(l, &Type::Bool)?;
                    self.check(r, &Type::Bool)?;
                    Ok(Type::Bool)
                }

                //   Γ ⊢ l : T   Γ ⊢ r : T   T is not a function type
                //  ------------------------------------------------- (T-Eq) for == !=
                //                  Γ ⊢ l == r : bool
                //
                // Equality on functions is excluded: deciding whether two
                // functions behave the same on every input is undecidable, and
                // comparing them by address would make `==` depend on how the
                // compiler happened to lay out code.
                BinOp::Eq | BinOp::Ne => {
                    let t = self.synth(l)?;
                    self.check(r, &t)?;
                    if let Type::Fn(..) = t {
                        return Err(Error::ty(
                            e.span,
                            format!("cannot compare functions with `{op}`"),
                        ));
                    }
                    Ok(Type::Bool)
                }
            },

            //   Γ ⊢ f : fn(T1, .., Tn) -> R    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
            //  ------------------------------------------------------------ (T-App)
            //                  Γ ⊢ f(a1, .., an) : R
            ExprKind::Call(f, args) => {
                let (params, ret) = match self.synth(f)? {
                    Type::Fn(params, ret) => (params, ret),
                    t => return Err(Error::ty(f.span, format!("`{t}` is not a function"))),
                };
                if params.len() != args.len() {
                    return Err(Error::ty(
                        e.span,
                        format!(
                            "this function takes {} argument{} but {} {} supplied",
                            params.len(),
                            if params.len() == 1 { "" } else { "s" },
                            args.len(),
                            if args.len() == 1 { "was" } else { "were" },
                        ),
                    ));
                }
                for (arg, param) in args.iter().zip(&params) {
                    self.check(arg, param)?;
                }
                Ok(*ret)
            }

            //   Γ ⊢ c : bool   Γ ⊢ a : T   Γ ⊢ b : T
            //  --------------------------------------- (T-If)
            //        Γ ⊢ if c { a } else { b } : T
            //
            //   Γ ⊢ c : bool   Γ ⊢ a : ()
            //  --------------------------- (T-If-NoElse)
            //     Γ ⊢ if c { a } : ()
            ExprKind::If(c, then, els) => {
                self.check(c, &Type::Bool)?;
                match els {
                    Some(els) => {
                        let t = self.synth(then)?;
                        let u = self.synth(els)?;
                        if t != u {
                            return Err(Error::ty(
                                result_span(els),
                                format!("`if` and `else` have different types: `{t}` and `{u}`"),
                            ));
                        }
                        Ok(t)
                    }
                    None => {
                        let t = self.synth(then)?;
                        if t != Type::Unit {
                            return Err(Error::ty(
                                result_span(then),
                                format!("`if` without `else` must have type `()`, found `{t}`"),
                            ));
                        }
                        Ok(Type::Unit)
                    }
                }
            }

            //   Γ ⊢ e1 : T1    Γ, x : T1 ⊢ rest : T
            //  ------------------------------------- (T-Let)
            //     Γ ⊢ { let x = e1; rest } : T
            //
            // `Γ, x : T1` is "Γ extended with x" (shadowing any earlier x).
            // Expression statements `e;` are typed and their value discarded.
            ExprKind::Block(stmts, tail) => {
                let scope = self.locals.len();
                for stmt in stmts {
                    match stmt {
                        Stmt::Let { name, ty, init } => {
                            let t = match ty {
                                Some(ann) => {
                                    let t = resolve(ann)?;
                                    self.check(init, &t)?;
                                    t
                                }
                                None => self.synth(init)?,
                            };
                            self.locals.push((name.name.clone(), t));
                        }
                        Stmt::Expr(e) => {
                            self.synth(e)?;
                        }
                    }
                }
                let t = match tail {
                    Some(tail) => self.synth(tail)?,
                    None => Type::Unit,
                };
                self.locals.truncate(scope);
                Ok(t)
            }
        }
    }
}

/// Where to point when an expression has the wrong type: for a block that's
/// its final expression, not the whole multi-line block.
fn result_span(e: &Expr) -> Span {
    match &e.kind {
        ExprKind::Block(_, Some(tail)) => result_span(tail),
        _ => e.span,
    }
}
