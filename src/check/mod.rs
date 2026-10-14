//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `int`, `bool`, `()` and function types, plus `let`, `if` and
//! top-level recursive functions (step 1), plus structs, enums and `match`
//! (step 2, see docs/02-adts.md), plus closures and type inference (step 3, see
//! docs/03-inference.md).
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
//!
//! Where a rule needs a type the program doesn't write down, like the `T` of
//! the parameter in `|x| x + 1`, the checker makes up a *type variable* and
//! works out what it must be from how it's used: `unify`.

mod escape;
mod lints;
mod unify;

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span, Warning};
use crate::exhaust::{self, Ctor, Pat, Redundant};
use crate::types::{AdtDef, Adts, Type, all_fields, struct_fields, variant_fields};
use escape::{Escape, Exit, Origin, bound_origin};
use lints::{Uses, mergeable, narrowable, unreachable_alt, unreachable_arm, unused};
use std::collections::{HashMap, HashSet};
use unify::Var;

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

/// Type-checks `prog`, adding any warnings to `warnings`, in source order.
/// Those found before an error are kept.
pub fn check(prog: &Program, warnings: &mut Vec<Warning>) -> Result<(), Error> {
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
        vars: Vec::new(),
        comparisons: Vec::new(),
        escapes: Vec::new(),
        uses: Vec::new(),
        warnings: Vec::new(),
    };
    let result = cx.bodies(prog).and_then(|()| check_main(prog));
    // What's dead depends on the whole program, so only once it all checks.
    if result.is_ok() {
        cx.dead_code(prog);
    }
    cx.warnings.sort_by_key(|w| w.span.start);
    warnings.append(&mut cx.warnings);
    result
}

/// Checks that there's a `main`, and that it takes nothing.
fn check_main(prog: &Program) -> Result<(), Error> {
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
    // Making that work takes a pointer, and a story for who frees it: that's
    // step 5, ownership.
    for (owner, span, t) in field_types {
        if contains(&adts, &t, &owner.name, &mut HashSet::new()) {
            return Err(Error::ty(
                span,
                format!(
                    "`{}` contains itself, so it would have infinite size \
                     (holding itself through a pointer comes in step 5)",
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
    all_fields(adts, ty)
        .iter()
        .any(|t| contains(adts, t, target, seen))
}

/// Adds the name of every struct and enum that `t` mentions to `out`.
fn adt_names(t: &Type, out: &mut HashSet<String>) {
    if let Type::Adt(name) = t {
        out.insert(name.clone());
    }
    for p in t.parts() {
        adt_names(p, out);
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

/// A variable in the local context Γ.
#[derive(Debug, Clone)]
struct Local {
    name: String,
    ty: Type,
    /// Where it's bound: one place, or one per alternative of an or-pattern.
    sites: Vec<Site>,
    /// Whether anything has read it, for the unused-variable warning.
    used: bool,
    /// Why it may hold a closure that must stay in its frame, if it may
    /// (see `escape`).
    origin: Option<Origin>,
}

/// Where a variable is bound. `shorthand` is for a struct pattern's `P { x }`,
/// which is short for `P { x: x }`.
#[derive(Debug, Clone, Copy)]
struct Site {
    span: Span,
    shorthand: bool,
}

impl Local {
    fn new(name: &str, ty: Type, span: Span) -> Local {
        Local {
            name: name.to_string(),
            ty,
            sites: vec![Site {
                span,
                shorthand: false,
            }],
            used: false,
            origin: None,
        }
    }

    /// A parameter, which may be a closure that captures.
    fn param(name: &str, ty: Type, span: Span) -> Local {
        Local {
            origin: Some(Origin::Param(name.to_string())),
            ..Local::new(name, ty, span)
        }
    }
}

/// Why an or-pattern alternative's bindings `b` don't have the same names as
/// the first alternative's `a`, if they don't.
fn or_mismatch(a: &[Local], b: &[Local]) -> Option<String> {
    for Local { name, .. } in a {
        if !b.iter().any(|l| l.name == *name) {
            return Some(format!("`{name}` is not bound in this alternative"));
        }
    }
    let extra = b.iter().find(|l| !a.iter().any(|m| m.name == l.name))?;
    Some(format!(
        "`{}` is bound in this alternative but not in the first",
        extra.name
    ))
}

/// "1 field", "2 fields".
fn plural(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}
struct Checker {
    adts: Adts,
    globals: HashMap<String, Type>,
    /// The local part of the context Γ, as a stack. Entering a scope pushes,
    /// leaving truncates (see `end_scope`). Searching from the top makes inner
    /// `let`s shadow outer ones.
    locals: Vec<Local>,
    /// Every type variable, indexed by `Type::Var`'s number.
    vars: Vec<Var>,
    /// `==` and `!=` on values whose type wasn't known yet, as the type, the
    /// span and the operator. Checked once the function's types are.
    comparisons: Vec<(Type, Span, BinOp)>,
    /// Values that may be closures leaving the frame that made them, checked
    /// once the function's types are known.
    escapes: Vec<Escape>,
    /// What each function checked so far refers to, in program order.
    uses: Vec<Uses>,
    warnings: Vec<Warning>,
}

impl Checker {
    /// Checks each function's body against its declared return type.
    fn bodies(&mut self, prog: &Program) -> Result<(), Error> {
        for f in &prog.fns {
            self.uses.push(Uses::default());
            for (i, p) in f.params.iter().enumerate() {
                if f.params[..i].iter().any(|q| q.name.name == p.name.name) {
                    return Err(Error::ty(
                        p.name.span,
                        format!("parameter `{}` is declared twice", p.name.name),
                    ));
                }
                let ty = resolve(&p.ty, &self.adts)?;
                self.mention(&ty);
                self.locals
                    .push(Local::param(&p.name.name, ty, p.name.span));
            }
            let ret = resolve(&f.ret, &self.adts)?;
            self.mention(&ret);
            let vars = self.vars.len();
            self.check(&f.body, &ret)?;
            self.leaves(&f.body, &ret, Exit::Return);
            self.end_scope(0);
            self.solve_all(vars)?;
        }
        Ok(())
    }

    /// Finds a variable's type, and notes that it was used.
    fn lookup(&mut self, name: &str) -> Option<Type> {
        match self.locals.iter_mut().rev().find(|l| l.name == name) {
            Some(local) => {
                local.used = true;
                Some(local.ty.clone())
            }
            None => {
                let ty = self.globals.get(name).cloned()?;
                self.uses().fns.insert(name.to_string());
                Some(ty)
            }
        }
    }

    /// What the function being checked refers to.
    fn uses(&mut self) -> &mut Uses {
        self.uses.last_mut().expect("checking outside a function")
    }

    /// Notes that the function being checked mentions type `t`.
    fn mention(&mut self, t: &Type) {
        adt_names(t, &mut self.uses().types);
    }

    /// Leaves a scope: drops the locals from `scope` on, warning about any
    /// that were never used. A name starting with `_` says that's on purpose.
    fn end_scope(&mut self, scope: usize) {
        for local in self.locals.split_off(scope) {
            if !local.used && !local.name.starts_with('_') {
                self.warnings.push(unused(&local));
            }
        }
    }

    /// Checks that `e` has type `want`.
    ///
    /// Mostly that's "synthesize, then unify". But this is *bidirectional*
    /// typing: checking is its own mode, which pushes the type that's wanted
    /// *down* into the expression, so its parts know their types before
    /// they're looked at.
    ///
    /// That matters for closures: in `apply(|p| p.x, ..)`, `p` gets its type
    /// from `apply`'s signature before the body reads `p.x`, which needs it.
    /// Blocks and `match` pass `want` on to their results, so that reaches
    /// closures inside them too.
    fn check(&mut self, e: &Expr, want: &Type) -> Result<(), Error> {
        let got = match &e.kind {
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), Some(want))?,
            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, Some(want))?,
            ExprKind::Lambda(l) => {
                let sig = match self.shallow(want) {
                    Type::Fn(params, ret) if params.len() == l.params.len() => Some((params, *ret)),
                    _ => None,
                };
                self.lambda(l, sig)?
            }
            _ => self.synth(e)?,
        };
        self.unify_or(&got, want, result_span(e), |got, want| {
            format!("expected `{want}`, found `{got}`")
        })
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
                // See `comparable` for why functions are excluded. If T isn't
                // known yet, as in `|a, b| a == b`, this premise waits until it
                // is, at the end of the function.
                BinOp::Eq | BinOp::Ne => {
                    let t = self.synth(l)?;
                    self.check(r, &t)?;
                    match self.shallow(&t) {
                        Type::Var(_) => self.comparisons.push((t, e.span, *op)),
                        t => self.comparable(&t, e.span, *op)?,
                    }
                    Ok(Type::Bool)
                }
            },

            //   Γ ⊢ f : fn(T1, .., Tn) -> R    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
            //  ------------------------------------------------------------ (T-App)
            //                  Γ ⊢ f(a1, .., an) : R
            //
            // If f's type isn't known yet, it's a function: one taking n
            // arguments of some types, returning some type.
            ExprKind::Call(f, args) => {
                let ft = self.synth(f)?;
                let (params, ret) = match self.shallow(&ft) {
                    Type::Fn(params, ret) => (params, *ret),
                    Type::Var(_) => {
                        let params: Vec<Type> = args
                            .iter()
                            .map(|a| {
                                let what = "the type of this argument".to_string();
                                self.fresh(a.span, what, "add a type annotation".into())
                            })
                            .collect();
                        let what = "the type of this call".to_string();
                        let ret = self.fresh(e.span, what, "add a type annotation".into());
                        let sig = Type::Fn(params.clone(), Box::new(ret.clone()));
                        self.unify(&ft, &sig)
                            .unwrap_or_else(|_| unreachable!("fresh variables fit anything"));
                        (params, ret)
                    }
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
                Ok(ret)
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
                        self.unify_or(&t, &u, result_span(els), |t, u| {
                            format!("`if` and `else` have different types: `{t}` and `{u}`")
                        })?;
                        Ok(t)
                    }
                    None => {
                        let t = self.synth(then)?;
                        self.unify_or(&t, &Type::Unit, result_span(then), |t, _| {
                            format!("`if` without `else` must have type `()`, found `{t}`")
                        })?;
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
                let ty = self.struct_type(name)?;
                let fields = struct_fields(&self.adts, &ty).expect("a struct");
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
                    self.leaves(&init.value, &fields[i].1, Exit::Store);
                }
                if let Some(i) = given.iter().position(|g| !g) {
                    return Err(Error::ty(
                        e.span,
                        format!("missing field `{}` in `{}`", fields[i].0, name.name),
                    ));
                }
                self.uses().types.insert(name.name.clone());
                self.uses().structs.insert(name.name.clone());
                Ok(ty)
            }

            //   Γ ⊢ e : S    struct S { .., f: T, .. }
            //  ---------------------------------------- (T-Field)
            //               Γ ⊢ e.f : T
            //
            // ...and *eliminated* by taking any one field out.
            //
            // Unlike most rules, this one needs e's type *now*: which struct
            // it is decides what T is. So it can't be a variable to solve
            // later.
            ExprKind::Field(x, field) => {
                let t = self.synth(x)?;
                let t = self.solved(&t);
                if let Type::Var(_) = t {
                    return Err(Error::ty(
                        x.span,
                        format!(
                            "the type of this must be known to read its field `{}`; \
                             add a type annotation",
                            field.name
                        ),
                    ));
                }
                let fields = struct_fields(&self.adts, &t).unwrap_or_default();
                match fields.into_iter().find(|(f, _)| *f == field.name) {
                    Some((_, ft)) => Ok(ft),
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
                let (t, i) = self.variant(ty, variant)?;
                let fields = variant_fields(&self.adts, &t, i);
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
                    self.leaves(arg, field, Exit::Store);
                }
                let (ty, variant) = (ty.name.clone(), variant.name.clone());
                self.uses().types.insert(ty.clone());
                self.uses().variants.insert((ty, variant));
                Ok(t)
            }

            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, None),

            ExprKind::Lambda(l) => self.lambda(l, None),
        }
    }

    //   Γ, x1 : T1, .., xn : Tn ⊢ e : R
    //  ---------------------------------- (T-Abs)
    //   Γ ⊢ |x1, .., xn| e : fn(T1, .., Tn) -> R
    //
    // Function *introduction*, the λ of the lambda calculus; T-App is its
    // elimination. The body sees Γ, so it can use the locals around it:
    // that's what makes it a closure.
    //
    // Nothing says what T1..Tn are. An annotation does, or else `sig`, the
    // signature wanted when checking (see `check`). Failing those, each is a
    // fresh variable, for `unify` to solve from how the body and the callers
    // use it.
    fn lambda(&mut self, l: &Lambda, sig: Option<(Vec<Type>, Type)>) -> Result<Type, Error> {
        let scope = self.locals.len();
        let mut params = Vec::new();
        for (i, p) in l.params.iter().enumerate() {
            let name = &p.name.name;
            if l.params[..i].iter().any(|q| q.name.name == *name) {
                return Err(Error::ty(
                    p.name.span,
                    format!("parameter `{name}` is declared twice"),
                ));
            }
            let wanted = sig.as_ref().map(|(ps, _)| ps[i].clone());
            let t = match (&p.ty, wanted) {
                (Some(ann), wanted) => self.annotation(ann, wanted)?,
                (None, Some(t)) => t,
                (None, None) => self.fresh(
                    p.name.span,
                    format!("the type of `{name}`"),
                    format!("annotate it, e.g. `|{name}: int|`"),
                ),
            };
            params.push(t.clone());
            self.locals.push(Local::param(name, t, p.name.span));
        }
        let wanted = sig.map(|(_, r)| r);
        let ret = match &l.ret {
            Some(ann) => Some(self.annotation(ann, wanted)?),
            None => wanted,
        };
        let ret = match ret {
            Some(r) => {
                self.check(&l.body, &r)?;
                r
            }
            None => self.synth(&l.body)?,
        };
        self.leaves(&l.body, &ret, Exit::Return);
        self.end_scope(scope);
        Ok(Type::Fn(params, Box::new(ret)))
    }

    /// The type a closure's annotation names, which must fit the one
    /// `wanted` from the context, if any.
    fn annotation(&mut self, ann: &TypeExpr, wanted: Option<Type>) -> Result<Type, Error> {
        let t = resolve(ann, &self.adts)?;
        self.mention(&t);
        if let Some(w) = wanted {
            self.unify_or(&t, &w, ann.span, |t, w| {
                format!("expected `{w}`, found `{t}`")
            })?;
        }
        Ok(t)
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
                            self.mention(&t);
                            self.check(init, &t)?;
                            t
                        }
                        None => self.synth(init)?,
                    };
                    let mut binds = Vec::new();
                    let p = self.pattern(pat, &t, &mut binds)?;
                    let origin = self.origin(init, &mut Vec::new()).map(|(o, _)| o);
                    if let Some(origin) = bound_origin(pat, origin) {
                        binds[0].origin = Some(origin);
                    }
                    let t = self.solved(&t);
                    // Only alternatives can be unreachable here. A whole `let`
                    // pattern only is when the value can't exist, like a
                    // `Never` parameter, and then nothing is wrong.
                    let p = [p];
                    let pats = std::slice::from_ref(pat);
                    for r in exhaust::redundant(&self.adts, &t, &p) {
                        match r {
                            Redundant::Alt(span) => self.warnings.push(unreachable_alt(span, pats)),
                            Redundant::Trim(span, lo, hi) => {
                                self.warnings.push(narrowable(span, lo, hi, pats));
                            }
                            Redundant::Arm(_) => {}
                        }
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
                    let what = match e.kind {
                        ExprKind::If(..) => "`if`",
                        ExprKind::Match(..) => "`match`",
                        _ => "block",
                    };
                    let (start, end) = (e.span.start, e.span.end);
                    self.unify_or(&t, &Type::Unit, result_span(e), |t, _| {
                        format!(
                            "expected `()`, found `{t}`: this {what} is a statement, so its \
                             value would be lost; add `;` to discard it, or wrap it in \
                             parentheses to use it in an expression"
                        )
                    })
                    .map_err(|err| {
                        err.with_fix(Fix::new(
                            "add `;` to discard the value",
                            vec![Edit::insert(end, ";")],
                        ))
                        .with_fix(Fix::new(
                            "wrap it in parentheses",
                            vec![Edit::insert(start, "("), Edit::insert(end, ")")],
                        ))
                    })?;
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
        self.end_scope(scope);
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
    // any other premise. Arms that can never match get a warning: they aren't
    // unsound, but they're always a mistake.
    fn match_expr(
        &mut self,
        e: &Expr,
        scrutinee: &Expr,
        arms: &[Arm],
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let t = self.synth(scrutinee)?;
        let origin = self.origin(scrutinee, &mut Vec::new()).map(|(o, _)| o);
        let mut pats = Vec::new();
        let mut binds = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            pats.push(self.pattern(&arm.pat, &t, &mut b)?);
            if let Some(origin) = bound_origin(&arm.pat, origin.clone()) {
                b[0].origin = Some(origin);
            }
            binds.push(b);
        }
        // The patterns may have told us what the scrutinee is.
        let t = self.solved(&t);
        let written: Vec<Pattern> = arms.iter().map(|a| a.pat.clone()).collect();
        for r in exhaust::redundant(&self.adts, &t, &pats) {
            self.warnings.push(match r {
                Redundant::Arm(i) => unreachable_arm(arms, i),
                Redundant::Alt(span) => unreachable_alt(span, &written),
                Redundant::Trim(span, lo, hi) => narrowable(span, lo, hi, &written),
            });
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
                    let r = r.clone();
                    let u = self.synth(&arm.body)?;
                    self.unify_or(&r, &u, result_span(&arm.body), |r, u| {
                        format!("`match` arms have different types: `{r}` and `{u}`")
                    })?;
                }
                (None, _) => result = Some(self.synth(&arm.body)?),
            }
            self.end_scope(scope);
        }
        // No arms and no type wanted: `let x = match n {};`. Its type is
        // whatever the rest of the function needs it to be.
        Ok(result.unwrap_or_else(|| {
            self.fresh(
                e.span,
                "the type of this `match`".into(),
                "it has no arms to tell, so give it a type, e.g. `let x: int = match ..`".into(),
            )
        }))
    }

    /// Checks that pattern `p` can match values of type `ty`, and adds the
    /// variables it binds to `binds`. This is the judgment `p : T ⊣ Δ` from
    /// T-Match, with `binds` as Δ.
    ///
    /// Returns the pattern in the reduced form the exhaustiveness checker
    /// uses.
    fn pattern(&mut self, p: &Pattern, ty: &Type, binds: &mut Vec<Local>) -> Result<Pat, Error> {
        Ok(match &p.kind {
            PatternKind::Wild => Pat::Wild,
            PatternKind::Bind(name) => {
                if binds.iter().any(|l| l.name == *name) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{name}` is bound more than once in the same pattern"),
                    ));
                }
                binds.push(Local::new(name, ty.clone(), p.span));
                Pat::Wild
            }
            PatternKind::Int(n) => {
                self.pattern_is(p, ty, Type::Int)?;
                Pat::Ctor(Ctor::Int(*n, *n), Vec::new())
            }
            PatternKind::Range(lo, hi, _) => {
                self.pattern_is(p, ty, Type::Int)?;
                if lo > hi {
                    return Err(Error::ty(
                        p.span,
                        "this range is empty, so it can never match",
                    ));
                }
                Pat::Range(*lo, *hi, p.span)
            }
            PatternKind::Bool(b) => {
                self.pattern_is(p, ty, Type::Bool)?;
                Pat::Ctor(Ctor::Bool(*b), Vec::new())
            }
            PatternKind::Unit => {
                self.pattern_is(p, ty, Type::Unit)?;
                Pat::Ctor(Ctor::Unit, Vec::new())
            }
            PatternKind::Variant(name, variant, args) => {
                let (t, i) = self.variant(name, variant)?;
                let fields = variant_fields(&self.adts, &t, i);
                self.pattern_is(p, ty, t)?;
                self.uses().types.insert(name.name.clone());
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
                    .zip(&fields)
                    .map(|(a, t)| self.pattern(a, t, binds))
                    .collect::<Result<_, _>>()?;
                Pat::Ctor(Ctor::Variant(i), args)
            }
            // Each alternative is checked separately, and they must all bind
            // the same Δ. Otherwise the arm's body could read a variable that
            // the alternative which actually matched never set.
            PatternKind::Or(alts) => {
                let outer = binds.len();
                let mut first: Option<Vec<Local>> = None;
                let mut pats = Vec::new();
                for alt in alts {
                    let mut b = binds.clone();
                    pats.push((self.pattern(alt, ty, &mut b)?, alt.span));
                    let new = b.split_off(outer);
                    match &mut first {
                        None => first = Some(new),
                        Some(want) => {
                            if let Some(msg) = or_mismatch(want, &new) {
                                return Err(Error::ty(alt.span, msg));
                            }
                            // The same variable, bound in one more place, so
                            // with the same type.
                            for l in want.iter_mut() {
                                let m = new.iter().find(|m| m.name == l.name);
                                let m = m.expect("or_mismatch checked the names");
                                self.unify_or(&l.ty, &m.ty, alt.span, |t, u| {
                                    format!(
                                        "`{}` is `{t}` in the first alternative but `{u}` in this one",
                                        l.name
                                    )
                                })?;
                                l.sites.extend(m.sites.iter().copied());
                            }
                        }
                    }
                }
                self.warnings.extend(mergeable(p, alts));
                binds.extend(first.unwrap_or_default());
                Pat::Or(pats)
            }
            PatternKind::Struct(name, field_pats, rest) => {
                let t = self.struct_type(name)?;
                let fields = struct_fields(&self.adts, &t).expect("a struct");
                self.pattern_is(p, ty, t)?;
                self.uses().types.insert(name.name.clone());
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
                    // `P { x }`: the parser made the pattern `x` out of the
                    // field name, so they're in the same place.
                    if fp.pat.span == fp.name.span
                        && let Some(site) = binds.last_mut().and_then(|l| l.sites.first_mut())
                    {
                        site.shorthand = true;
                    }
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

    /// Unifies the type `ty` a pattern should match with the type `found` it
    /// does match.
    fn pattern_is(&mut self, p: &Pattern, ty: &Type, found: Type) -> Result<(), Error> {
        self.unify_or(&found, ty, p.span, |found, ty| {
            format!("expected `{ty}`, found `{found}`")
        })
    }

    /// The type a struct literal or pattern names, which must be a struct.
    fn struct_type(&self, name: &Ident) -> Result<Type, Error> {
        match self.adts.get(&name.name) {
            Some(AdtDef::Struct(_)) => Ok(Type::Adt(name.name.clone())),
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

    /// Looks up `E::V`, returning the enum's type and the variant's index.
    fn variant(&self, ty: &Ident, variant: &Ident) -> Result<(Type, usize), Error> {
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
            .position(|(v, _)| *v == variant.name)
            .map(|i| (Type::Adt(ty.name.clone()), i))
            .ok_or_else(|| {
                Error::ty(
                    variant.span,
                    format!("`{}` has no variant `{}`", ty.name, variant.name),
                )
            })
    }

    /// The rest of T-Eq's premise: `t`, which is known, contains no
    /// function type.
    ///
    /// Equality on functions is excluded: deciding whether two functions
    /// behave the same on every input is undecidable, and comparing them by
    /// address would make `==` depend on how the compiler happened to lay out
    /// code. Structs and enums compare field by field, so they're excluded if
    /// any field is (or holds) a function.
    fn comparable(&self, t: &Type, span: Span, op: BinOp) -> Result<(), Error> {
        if let Type::Fn(..) = t {
            return Err(Error::ty(
                span,
                format!("cannot compare functions with `{op}`"),
            ));
        }
        if self.holds_fn(t) {
            return Err(Error::ty(
                span,
                format!("cannot compare `{t}` with `{op}`: it contains a function"),
            ));
        }
        Ok(())
    }

    /// Is `t` a function, or a struct or enum with one somewhere inside?
    /// This terminates because types can't contain themselves.
    fn holds_fn(&self, t: &Type) -> bool {
        match t {
            Type::Fn(..) => true,
            Type::Adt(_) => all_fields(&self.adts, t).iter().any(|t| self.holds_fn(t)),
            // Fields' types are always known.
            Type::Int | Type::Bool | Type::Unit | Type::Var(_) => false,
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
