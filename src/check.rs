//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `int`, `bool`, `()` and function types, plus `let`, `if` and
//! top-level recursive functions (step 1), plus structs, enums and `match`
//! (step 2, see docs/02-adts.md).
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
use crate::exhaust::{self, Ctor, Pat, Redundant};
use crate::types::{AdtDef, Adts, Type};
use std::collections::{HashMap, HashSet};

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
    let adts = collect_adts(prog)?;
    let mut globals = HashMap::new();
    for &b in BUILTINS {
        globals.insert(b.to_string(), builtin_type(b));
    }

    // Pass 1: collect every function's signature first, so bodies can refer to
    // functions defined later in the file (and to themselves: recursion).
    for f in &prog.fns {
        let params = f
            .params
            .iter()
            .map(|p| resolve(&p.ty, &adts))
            .collect::<Result<Vec<_>, _>>()?;
        let ret = resolve(&f.ret, &adts)?;
        let ty = Type::Fn(params, Box::new(ret));
        if globals.insert(f.name.name.clone(), ty).is_some() {
            return Err(Error::ty(
                f.name.span,
                format!("`{}` is already defined", f.name.name),
            ));
        }
    }

    // Pass 2: check each body against its declared return type.
    let mut cx = Checker {
        adts,
        globals,
        locals: Vec::new(),
    };
    for f in &prog.fns {
        cx.locals.clear();
        for (i, p) in f.params.iter().enumerate() {
            if f.params[..i].iter().any(|q| q.name.name == p.name.name) {
                return Err(Error::ty(
                    p.name.span,
                    format!("parameter `{}` is declared twice", p.name.name),
                ));
            }
            cx.locals
                .push((p.name.name.clone(), resolve(&p.ty, &cx.adts)?));
        }
        let ret = resolve(&f.ret, &cx.adts)?;
        cx.check(&f.body, &ret)?;
    }

    match prog.fns.iter().find(|f| f.name.name == "main") {
        None => Err(Error::ty(Span::new(0, 0), "program has no `main` function")),
        Some(main) if !main.params.is_empty() => {
            Err(Error::ty(main.name.span, "`main` must not take parameters"))
        }
        Some(_) => Ok(()),
    }
}

/// Builds the table of structs and enums. Rejects duplicate names, fields and
/// variants, and types that contain themselves.
fn collect_adts(prog: &Program) -> Result<Adts, Error> {
    // All names first, so declarations can mention each other in any order.
    let mut names = HashMap::new();
    for d in &prog.types {
        let name = d.name();
        if matches!(name.name.as_str(), "int" | "bool") {
            return Err(Error::ty(
                name.span,
                format!("`{}` is a built-in type", name.name),
            ));
        }
        if names.insert(name.name.clone(), ()).is_some() {
            return Err(Error::ty(
                name.span,
                format!("type `{}` is already defined", name.name),
            ));
        }
    }

    let mut adts = Adts::new();
    // Every field's type, with where it was written, for the recursion check.
    let mut field_types = Vec::new();
    for d in &prog.types {
        let def = match d {
            TypeDecl::Struct(s) => {
                let mut fields: Vec<(String, Type)> = Vec::new();
                for f in &s.fields {
                    if fields.iter().any(|(n, _)| *n == f.name.name) {
                        return Err(Error::ty(
                            f.name.span,
                            format!("field `{}` is declared twice", f.name.name),
                        ));
                    }
                    let t = resolve(&f.ty, &names)?;
                    field_types.push((&s.name, f.ty.span, t.clone()));
                    fields.push((f.name.name.clone(), t));
                }
                AdtDef::Struct(fields)
            }
            TypeDecl::Enum(e) => {
                let mut variants: Vec<(String, Vec<Type>)> = Vec::new();
                for v in &e.variants {
                    if variants.iter().any(|(n, _)| *n == v.name.name) {
                        return Err(Error::ty(
                            v.name.span,
                            format!("variant `{}` is declared twice", v.name.name),
                        ));
                    }
                    let mut tys = Vec::new();
                    for f in &v.fields {
                        let t = resolve(f, &names)?;
                        field_types.push((&e.name, f.span, t.clone()));
                        tys.push(t);
                    }
                    variants.push((v.name.name.clone(), tys));
                }
                AdtDef::Enum(variants)
            }
        };
        adts.insert(d.name().name.clone(), def);
    }

    // A struct that holds itself, even via other types, would need infinite
    // memory: `struct A { a: A }` has the size of an int plus... itself.
    // Making that work takes a pointer (and a story for who frees it), plus
    // a theory of recursive types. That's step 5.
    for (owner, span, t) in field_types {
        if contains(&adts, &t, &owner.name, &mut HashSet::new()) {
            return Err(Error::ty(
                span,
                format!(
                    "`{}` contains itself, so it would have infinite size \
                     (recursive types come in step 5)",
                    owner.name
                ),
            ));
        }
    }
    Ok(adts)
}

/// Does a value of type `ty` hold a `target`, at any depth? Function types
/// don't count: a function value is a code address, not the values its
/// signature mentions.
fn contains(adts: &Adts, ty: &Type, target: &str, seen: &mut HashSet<String>) -> bool {
    let Type::Adt(name) = ty else { return false };
    if name == target {
        return true;
    }
    if !seen.insert(name.clone()) {
        return false;
    }
    field_types(&adts[name]).any(|t| contains(adts, t, target, seen))
}

/// The types of every field of every variant.
fn field_types(def: &AdtDef) -> Box<dyn Iterator<Item = &Type> + '_> {
    match def {
        AdtDef::Struct(fields) => Box::new(fields.iter().map(|(_, t)| t)),
        AdtDef::Enum(variants) => Box::new(variants.iter().flat_map(|(_, ts)| ts)),
    }
}

/// Turns a type as written (`TypeExpr`) into a type as understood (`Type`).
/// `adts` holds the names of the program's structs and enums.
fn resolve<V>(t: &TypeExpr, adts: &HashMap<String, V>) -> Result<Type, Error> {
    match &t.kind {
        TypeExprKind::Named(name) => match name.as_str() {
            "int" => Ok(Type::Int),
            "bool" => Ok(Type::Bool),
            _ if adts.contains_key(name) => Ok(Type::Adt(name.clone())),
            _ => Err(Error::ty(t.span, format!("unknown type `{name}`"))),
        },
        TypeExprKind::Unit => Ok(Type::Unit),
        TypeExprKind::Fn(params, ret) => Ok(Type::Fn(
            params
                .iter()
                .map(|p| resolve(p, adts))
                .collect::<Result<_, _>>()?,
            Box::new(resolve(ret, adts)?),
        )),
    }
}

/// Why an or-pattern alternative's bindings `b` disagree with the first
/// alternative's `a`, if they do.
fn or_mismatch(a: &[(String, Type)], b: &[(String, Type)]) -> Option<String> {
    for (name, t) in a {
        match b.iter().find(|(n, _)| n == name) {
            None => return Some(format!("`{name}` is not bound in this alternative")),
            Some((_, u)) if u != t => {
                return Some(format!(
                    "`{name}` is `{t}` in the first alternative but `{u}` in this one"
                ));
            }
            Some(_) => {}
        }
    }
    let (extra, _) = b.iter().find(|(n, _)| !a.iter().any(|(m, _)| m == n))?;
    Some(format!(
        "`{extra}` is bound in this alternative but not in the first"
    ))
}

fn unreachable_alt(span: Span) -> Error {
    Error::ty(
        span,
        "unreachable alternative: the patterns before it already match everything it would",
    )
}

/// "1 field", "2 fields".
fn plural(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

struct Checker {
    adts: Adts,
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
    /// is mostly "synthesize, then compare". In step 3 (bidirectional typing)
    /// checking becomes its own mode that can push type information *down*
    /// into an expression.
    ///
    /// Blocks and `match` already push `want` down to their results. It makes
    /// no difference except for one expression: `match x {}` on an enum with
    /// no variants. It has no arms to compute a type from, but since it can
    /// never run, it can have whatever type is wanted (see docs/02-adts.md).
    fn check(&mut self, e: &Expr, want: &Type) -> Result<(), Error> {
        let got = match &e.kind {
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), Some(want))?,
            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, Some(want))?,
            _ => self.synth(e)?,
        };
        if got == *want {
            Ok(())
        } else {
            Err(Error::ty(
                result_span(e),
                format!("expected `{want}`, found `{got}`"),
            ))
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

                //   Γ ⊢ l : T   Γ ⊢ r : T   T contains no function type
                //  ---------------------------------------------------- (T-Eq) for == !=
                //                  Γ ⊢ l == r : bool
                //
                // Equality on functions is excluded: deciding whether two
                // functions behave the same on every input is undecidable, and
                // comparing them by address would make `==` depend on how the
                // compiler happened to lay out code. Structs and enums compare
                // field by field, so they're excluded if any field is (or
                // holds) a function.
                BinOp::Eq | BinOp::Ne => {
                    let t = self.synth(l)?;
                    self.check(r, &t)?;
                    if let Type::Fn(..) = t {
                        return Err(Error::ty(
                            e.span,
                            format!("cannot compare functions with `{op}`"),
                        ));
                    }
                    if self.holds_fn(&t) {
                        return Err(Error::ty(
                            e.span,
                            format!("cannot compare `{t}` with `{op}`: it contains a function"),
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

            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), None),

            //   struct S { f1: T1, .., fn: Tn }    Γ ⊢ e1 : T1  ..  Γ ⊢ en : Tn
            //  ---------------------------------------------------------------- (T-Struct)
            //               Γ ⊢ S { f1: e1, .., fn: en } : S
            //
            // A product type is *introduced* by giving every field.
            ExprKind::StructLit(name, inits) => {
                let fields = self.struct_def(name)?.to_vec();
                let mut given = vec![false; fields.len()];
                for init in inits {
                    let Some(i) = fields.iter().position(|(f, _)| *f == init.name.name) else {
                        return Err(Error::ty(
                            init.name.span,
                            format!("`{}` has no field `{}`", name.name, init.name.name),
                        ));
                    };
                    if given[i] {
                        return Err(Error::ty(
                            init.name.span,
                            format!("field `{}` is given twice", init.name.name),
                        ));
                    }
                    given[i] = true;
                    self.check(&init.value, &fields[i].1)?;
                }
                if let Some(i) = given.iter().position(|g| !g) {
                    return Err(Error::ty(
                        e.span,
                        format!("missing field `{}` in `{}`", fields[i].0, name.name),
                    ));
                }
                Ok(Type::Adt(name.name.clone()))
            }

            //   Γ ⊢ e : S    struct S { .., f: T, .. }
            //  ---------------------------------------- (T-Field)
            //               Γ ⊢ e.f : T
            //
            // ...and *eliminated* by taking any one field out.
            ExprKind::Field(x, field) => {
                let t = self.synth(x)?;
                let fields = match &t {
                    Type::Adt(name) => match &self.adts[name] {
                        AdtDef::Struct(fields) => fields.as_slice(),
                        AdtDef::Enum(_) => &[],
                    },
                    _ => &[],
                };
                match fields.iter().find(|(f, _)| *f == field.name) {
                    Some((_, ft)) => Ok(ft.clone()),
                    None => Err(Error::ty(
                        field.span,
                        format!("`{t}` has no field `{}`", field.name),
                    )),
                }
            }

            //   enum E { .., V(T1, .., Tn), .. }    Γ ⊢ e1 : T1  ..  Γ ⊢ en : Tn
            //  ----------------------------------------------------------------- (T-Variant)
            //                   Γ ⊢ E::V(e1, .., en) : E
            //
            // A sum type is *introduced* by picking one variant and giving its
            // fields. Its eliminator is `match`.
            ExprKind::Variant(ty, variant, args) => {
                let (_, fields) = self.variant(ty, variant)?;
                let fields = fields.to_vec();
                if fields.len() != args.len() {
                    return Err(Error::ty(
                        e.span,
                        format!(
                            "`{}::{}` has {} but {} {} supplied",
                            ty.name,
                            variant.name,
                            plural(fields.len(), "field"),
                            args.len(),
                            if args.len() == 1 { "was" } else { "were" },
                        ),
                    ));
                }
                for (arg, field) in args.iter().zip(&fields) {
                    self.check(arg, field)?;
                }
                Ok(Type::Adt(ty.name.clone()))
            }

            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, None),
        }
    }

    //   Γ ⊢ e1 : T1    Γ, x : T1 ⊢ rest : T
    //  ------------------------------------- (T-Let)
    //     Γ ⊢ { let x = e1; rest } : T
    //
    // `Γ, x : T1` is "Γ extended with x" (shadowing any earlier x).
    // Expression statements `e;` are typed and their value discarded. An
    // `if`/`match`/block statement without the `;` must be `()`.
    //
    // With a pattern, `let p = e1;` extends Γ with every variable `p` binds.
    // `p` must be *irrefutable* (match every value of type T1): there's no
    // other branch to take if it doesn't.
    //
    // `want` is the expected type of the result, when checking (see `check`).
    fn block(
        &mut self,
        stmts: &[Stmt],
        tail: Option<&Expr>,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let scope = self.locals.len();
        for stmt in stmts {
            match stmt {
                Stmt::Let { pat, ty, init } => {
                    let t = match ty {
                        Some(ann) => {
                            let t = resolve(ann, &self.adts)?;
                            self.check(init, &t)?;
                            t
                        }
                        None => self.synth(init)?,
                    };
                    let mut binds = Vec::new();
                    let p = self.pattern(pat, &t, &mut binds)?;
                    // Only alternatives can be unreachable here. A whole `let`
                    // pattern only is when the value can't exist, like a
                    // `Never` parameter, and then nothing is wrong.
                    let p = [p];
                    if let Some(Redundant::Alt(span)) = exhaust::redundant(&self.adts, &t, &p) {
                        return Err(unreachable_alt(span));
                    }
                    if let Some(missing) = exhaust::missing(&self.adts, &t, &p) {
                        return Err(Error::ty(
                            pat.span,
                            format!(
                                "refutable pattern in `let`: `{missing}` not covered; use `match` instead"
                            ),
                        ));
                    }
                    self.locals.extend(binds);
                }
                Stmt::Expr(e) => {
                    self.synth(e)?;
                }
                // Rust's rule, and the reason for it: in
                // `if c { 10 } else { 20 } - x` the `if` is its own statement,
                // and its value would silently be thrown away.
                Stmt::BlockLike(e) => {
                    let t = self.synth(e)?;
                    if t != Type::Unit {
                        let what = match e.kind {
                            ExprKind::If(..) => "`if`",
                            ExprKind::Match(..) => "`match`",
                            _ => "block",
                        };
                        return Err(Error::ty(
                            result_span(e),
                            format!(
                                "expected `()`, found `{t}`: this {what} is a statement, so its \
                                 value would be lost; add `;` to discard it, or wrap it in \
                                 parentheses to use it in an expression"
                            ),
                        ));
                    }
                }
            }
        }
        let t = match (tail, want) {
            (Some(tail), Some(want)) => {
                self.check(tail, want)?;
                want.clone()
            }
            (Some(tail), None) => self.synth(tail)?,
            (None, _) => Type::Unit,
        };
        self.locals.truncate(scope);
        Ok(t)
    }

    //   Γ ⊢ e : T    pi : T ⊣ Δi    Γ, Δi ⊢ ei : R  (each arm i)    p1..pn cover T
    //  ---------------------------------------------------------------------------- (T-Match)
    //                    Γ ⊢ match e { p1 => e1, .., pn => en } : R
    //
    // `p : T ⊣ Δ` reads "pattern p matches values of type T and binds the
    // variables Δ" (see `pattern`). The last premise is exhaustiveness: every
    // value of type T matches some pi. Without it, a value matching no arm
    // would leave the program stuck, so it's as much a part of soundness as
    // any other premise. Arms that can never match are rejected too: they
    // aren't unsound, but they're always a mistake.
    fn match_expr(
        &mut self,
        e: &Expr,
        scrutinee: &Expr,
        arms: &[Arm],
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let t = self.synth(scrutinee)?;
        let mut pats = Vec::new();
        let mut binds = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            pats.push(self.pattern(&arm.pat, &t, &mut b)?);
            binds.push(b);
        }
        match exhaust::redundant(&self.adts, &t, &pats) {
            Some(Redundant::Arm(i)) => {
                return Err(Error::ty(
                    arms[i].pat.span,
                    "unreachable pattern: the arms above already match everything it would",
                ));
            }
            Some(Redundant::Alt(span)) => return Err(unreachable_alt(span)),
            None => {}
        }
        if let Some(missing) = exhaust::missing(&self.adts, &t, &pats) {
            return Err(Error::ty(
                scrutinee.span,
                format!("non-exhaustive `match`: `{missing}` not covered"),
            ));
        }

        let mut result = want.cloned();
        for (arm, b) in arms.iter().zip(binds) {
            let scope = self.locals.len();
            self.locals.extend(b);
            match (&result, want) {
                (Some(r), Some(_)) => self.check(&arm.body, &r.clone())?,
                (Some(r), None) => {
                    let u = self.synth(&arm.body)?;
                    if u != *r {
                        return Err(Error::ty(
                            result_span(&arm.body),
                            format!("`match` arms have different types: `{r}` and `{u}`"),
                        ));
                    }
                }
                (None, _) => result = Some(self.synth(&arm.body)?),
            }
            self.locals.truncate(scope);
        }
        result.ok_or_else(|| {
            Error::ty(
                e.span,
                "a `match` with no arms needs a known type here, e.g. `let x: int = match ..`",
            )
        })
    }

    /// Checks that pattern `p` can match values of type `ty`, and adds the
    /// variables it binds to `binds`. This is the judgment `p : T ⊣ Δ` from
    /// T-Match, with `binds` as Δ.
    ///
    /// Returns the pattern in the reduced form the exhaustiveness checker
    /// uses.
    fn pattern(
        &self,
        p: &Pattern,
        ty: &Type,
        binds: &mut Vec<(String, Type)>,
    ) -> Result<Pat, Error> {
        let expect = |found: Type| {
            if found == *ty {
                Ok(())
            } else {
                Err(Error::ty(
                    p.span,
                    format!("expected `{ty}`, found `{found}`"),
                ))
            }
        };
        Ok(match &p.kind {
            PatternKind::Wild => Pat::Wild,
            PatternKind::Bind(name) => {
                if binds.iter().any(|(n, _)| n == name) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{name}` is bound more than once in the same pattern"),
                    ));
                }
                binds.push((name.clone(), ty.clone()));
                Pat::Wild
            }
            PatternKind::Int(n) => {
                expect(Type::Int)?;
                Pat::Ctor(Ctor::Int(*n), Vec::new())
            }
            PatternKind::Bool(b) => {
                expect(Type::Bool)?;
                Pat::Ctor(Ctor::Bool(*b), Vec::new())
            }
            PatternKind::Unit => {
                expect(Type::Unit)?;
                Pat::Ctor(Ctor::Unit, Vec::new())
            }
            PatternKind::Variant(name, variant, args) => {
                let (i, fields) = self.variant(name, variant)?;
                expect(Type::Adt(name.name.clone()))?;
                if fields.len() != args.len() {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "`{}::{}` has {}, but this pattern has {}",
                            name.name,
                            variant.name,
                            plural(fields.len(), "field"),
                            args.len(),
                        ),
                    ));
                }
                let args = args
                    .iter()
                    .zip(fields)
                    .map(|(a, t)| self.pattern(a, t, binds))
                    .collect::<Result<_, _>>()?;
                Pat::Ctor(Ctor::Variant(i), args)
            }
            // Each alternative is checked separately, and they must all bind
            // the same Δ. Otherwise the arm's body could read a variable that
            // the alternative which actually matched never set.
            PatternKind::Or(alts) => {
                let outer = binds.len();
                let mut first: Option<Vec<(String, Type)>> = None;
                let mut pats = Vec::new();
                for alt in alts {
                    let mut b = binds.clone();
                    pats.push((self.pattern(alt, ty, &mut b)?, alt.span));
                    let new = b.split_off(outer);
                    match &first {
                        None => first = Some(new),
                        Some(want) => {
                            if let Some(msg) = or_mismatch(want, &new) {
                                return Err(Error::ty(alt.span, msg));
                            }
                        }
                    }
                }
                binds.extend(first.unwrap_or_default());
                Pat::Or(pats)
            }
            PatternKind::Struct(name, field_pats, rest) => {
                let fields = self.struct_def(name)?;
                expect(Type::Adt(name.name.clone()))?;
                // Fields left out of the pattern match anything.
                let mut args = vec![Pat::Wild; fields.len()];
                let mut given = vec![false; fields.len()];
                for fp in field_pats {
                    let Some(i) = fields.iter().position(|(f, _)| *f == fp.name.name) else {
                        return Err(Error::ty(
                            fp.name.span,
                            format!("`{}` has no field `{}`", name.name, fp.name.name),
                        ));
                    };
                    if given[i] {
                        return Err(Error::ty(
                            fp.name.span,
                            format!("field `{}` is mentioned twice", fp.name.name),
                        ));
                    }
                    given[i] = true;
                    args[i] = self.pattern(&fp.pat, &fields[i].1, binds)?;
                }
                if let Some(i) = given.iter().position(|g| !g)
                    && !rest
                {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "pattern does not mention field `{}`; add it, or `..` to ignore the rest",
                            fields[i].0
                        ),
                    ));
                }
                Pat::Ctor(Ctor::Struct, args)
            }
        })
    }

    fn struct_def(&self, name: &Ident) -> Result<&[(String, Type)], Error> {
        match self.adts.get(&name.name) {
            Some(AdtDef::Struct(fields)) => Ok(fields),
            Some(AdtDef::Enum(_)) => Err(Error::ty(
                name.span,
                format!("`{}` is an enum, not a struct", name.name),
            )),
            None => Err(Error::ty(
                name.span,
                format!("unknown type `{}`", name.name),
            )),
        }
    }

    /// Looks up `E::V`, returning the variant's index and field types.
    fn variant(&self, ty: &Ident, variant: &Ident) -> Result<(usize, &[Type]), Error> {
        let variants = match self.adts.get(&ty.name) {
            Some(AdtDef::Enum(variants)) => variants,
            Some(AdtDef::Struct(_)) => {
                return Err(Error::ty(
                    ty.span,
                    format!("`{}` is a struct, not an enum", ty.name),
                ));
            }
            None => return Err(Error::ty(ty.span, format!("unknown type `{}`", ty.name))),
        };
        variants
            .iter()
            .enumerate()
            .find(|(_, (v, _))| *v == variant.name)
            .map(|(i, (_, fields))| (i, fields.as_slice()))
            .ok_or_else(|| {
                Error::ty(
                    variant.span,
                    format!("`{}` has no variant `{}`", ty.name, variant.name),
                )
            })
    }

    /// Is `t` a function, or a struct or enum with one somewhere inside?
    /// This terminates because types can't contain themselves.
    fn holds_fn(&self, t: &Type) -> bool {
        match t {
            Type::Fn(..) => true,
            Type::Adt(name) => field_types(&self.adts[name]).any(|t| self.holds_fn(t)),
            Type::Int | Type::Bool | Type::Unit => false,
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
