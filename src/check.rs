//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `int`, `bool`, `()` and function types, plus `let`, `if` and
//! recursive functions (step 1), plus structs, enums and `match`
//! (step 2, see docs/02-adts.md), plus closures, nested functions and types,
//! and type inference (step 3, see docs/03-inference.md), plus generics
//! (step 4, see docs/04-generics.md).
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
//! context Γ maps variable names to their types (here: `locals` + `scopes`).
//!
//! Every function here is *syntax-directed*: for each form of expression
//! exactly one rule applies, so the rules can be read directly as a recursive
//! algorithm. That's not true of every type system (see docs/01-stlc.md).
//!
//! Where a rule needs a type the program doesn't write down, like the `T` of
//! the parameter in `|x| x + 1`, the checker makes up a *type variable* and
//! works out what it must be from how it's used: `unify`.

mod escape;
mod generics;
mod info;
mod lints;
mod names;
mod scope;
mod unify;

pub(crate) use generics::{TypeName, resolve};
pub use info::{Info, Instance, Note, NoteKind};
pub use scope::source_name;

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span, Warning};
use crate::exhaust::{self, Ctor, Pat, Redundant};
use crate::types::{
    AdtDef, AdtKind, Adts, Scheme, Subst, Type, all_fields, struct_fields, variant_fields,
};
use escape::{Escape, Exit, Origin, bound_origin};
use generics::Use;
use info::Defs;
use lints::{Uses, mergeable, narrowable, unreachable_alt, unreachable_arm, unused, unused_doc};
use names::{Scoped, TypeDef};
use scope::{Binding, FnDef, FnId, FnScope, to_closure};
use std::collections::{HashMap, HashSet};
use unify::Var;

/// Built-in functions, available everywhere. Shared with the interpreter.
pub const BUILTINS: &[&str] = &["print", "print_bool"];

fn builtin_type(name: &str) -> Type {
    match name {
        // print : fn(int) -> ()
        "print" => Type::func(vec![Type::Int], Type::Unit),
        // print_bool : fn(bool) -> ()
        "print_bool" => Type::func(vec![Type::Bool], Type::Unit),
        _ => unreachable!("unknown builtin `{name}`"),
    }
}

/// Type-checks `prog`, adding any warnings to `warnings`, in source order.
/// Those found before an error are kept, and so is what was learned about
/// the program up to it.
pub fn check(prog: &Program, warnings: &mut Vec<Warning>) -> (Info, Result<(), Error>) {
    let mut cx = Checker::new(prog, false);
    cx.warnings.extend(prog.stray_docs.iter().map(unused_doc));
    let result = cx.program(prog);
    cx.type_notes(prog);
    cx.info.types = cx.defs.types.clone();
    cx.warnings.sort_by_key(|w| w.span.start);
    warnings.append(&mut cx.warnings);
    (cx.info, result)
}

/// Checks `prog` like `check`, but without the warnings, and with the name
/// of the function the error is in, if it's in one. For checking what
/// monomorphisation made (see `mono.rs`).
pub fn recheck(prog: &Program) -> Result<(), (Option<String>, Error)> {
    let mut cx = Checker::new(prog, true);
    cx.program(prog).map_err(|e| (cx.failed_in.take(), e))
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

/// Calls `f` with the name and type arguments of every struct and enum in
/// `t`, at any depth.
fn each_adt<'t>(t: &'t Type, f: &mut impl FnMut(&'t str, &'t [Type])) {
    if let Type::Adt(name, args) = t {
        f(name, args);
    }
    for p in t.parts() {
        each_adt(p, f);
    }
}

/// Does a value of type `ty` hold a `target`, at any depth? Function types
/// don't count: a function value is a code address, not the values its
/// signature mentions. `struct A { b: Box<A> }` counts, through `Box`'s
/// field of type `T`.
///
/// This terminates because polymorphic recursion is ruled out first, so only
/// finitely many types can turn up in here.
fn contains(adts: &Adts, ty: &Type, target: &str, seen: &mut HashSet<Type>) -> bool {
    let Type::Adt(name, _) = ty else { return false };
    if name == target {
        return true;
    }
    if !seen.insert(ty.clone()) {
        return false;
    }
    all_fields(adts, ty)
        .iter()
        .any(|t| contains(adts, t, target, seen))
}

/// Adds the name of every struct and enum that `t` mentions to `out`.
fn adt_names(t: &Type, out: &mut HashSet<String>) {
    each_adt(t, &mut |name, _| {
        out.insert(name.to_string());
    });
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
    /// What every struct and enum is made of, by symbol.
    adts: Adts,
    /// How many type arguments each struct and enum takes, by symbol.
    arities: HashMap<String, usize>,
    /// Every struct and enum, top-level and local, in the order of
    /// `Program::all_types`.
    types: Vec<TypeDef>,
    /// What each name in a type refers to, by its span (see `names.rs`).
    /// `None` when checking what monomorphisation made, which has
    /// everything at the top level, named by its symbol, and where a copy
    /// has the spans of what it's a copy of, all through a type, so spans
    /// don't tell names apart. Names are looked up as they are then.
    type_names: Option<HashMap<Span, Scoped>>,
    /// The symbol of every function, by `FnId`.
    fn_symbols: Vec<String>,
    /// Every function, top-level and nested, by `FnId`.
    fns: Vec<FnDef>,
    /// Each nested function's id, by where its name is declared.
    nested_ids: HashMap<Span, FnId>,
    /// The functions in scope, as a stack: the top-level ones, then those
    /// of each block around what's being checked that declares any. With
    /// the built-ins, the global part of the context Γ (see `scope.rs`).
    scopes: Vec<FnScope>,
    /// The function being checked.
    current: Option<FnId>,
    /// The type parameters of the function being checked.
    type_params: Vec<String>,
    /// The local part of the context Γ, as a stack. Entering a scope pushes,
    /// leaving truncates (see `end_scope`). Searching from the top makes inner
    /// `let`s shadow outer ones.
    locals: Vec<Local>,
    /// Where the function being checked has its locals from. Those below
    /// are of functions it's nested in, which it can't capture.
    barrier: usize,
    /// How to fix a capture by the nested function being checked, if
    /// there's a way, and the index of the first local it works for.
    closure_fix: Option<(usize, Fix)>,
    /// Every type variable, indexed by `Type::Var`'s number.
    vars: Vec<Var>,
    /// `==` and `!=` on values whose type wasn't fully known yet, as the
    /// type, the span and the operator. Checked once the function's types
    /// are.
    comparisons: Vec<(Type, Span, BinOp)>,
    /// Values that may be closures leaving the frame that made them, checked
    /// once the function's types are known.
    escapes: Vec<Escape>,
    /// What each function checked so far refers to, by `FnId`.
    uses: Vec<Uses>,
    warnings: Vec<Warning>,
    /// Where things are declared, for `info`.
    defs: Defs,
    /// What's been learned about the program.
    info: Info,
    /// Uses of generic things in the function being checked, with type
    /// arguments that may still have variables in them. `finish_notes`
    /// moves them to `info`.
    pending: Vec<(Span, Instance)>,
    /// The function the first error was in, if it was in one.
    failed_in: Option<String>,
}

impl Checker {
    /// `mono` says whether `prog` is what monomorphisation made.
    fn new(prog: &Program, mono: bool) -> Checker {
        let fn_symbols = scope::symbols(&prog.all_fns());
        let (types, type_names) = names::names(prog, &prog.all_fns(), &fn_symbols);
        let type_names = (!mono).then_some(type_names);
        let mut info = Info::default();
        for (span, name) in type_names.iter().flatten() {
            if let Scoped::Adt(symbol) = name {
                info.symbols.insert(*span, symbol.clone());
            }
        }
        Checker {
            adts: Adts::new(),
            arities: HashMap::new(),
            defs: Defs::new(prog, &types),
            types,
            type_names,
            fn_symbols,
            fns: Vec::new(),
            nested_ids: HashMap::new(),
            scopes: Vec::new(),
            current: None,
            type_params: Vec::new(),
            locals: Vec::new(),
            barrier: 0,
            closure_fix: None,
            vars: Vec::new(),
            comparisons: Vec::new(),
            escapes: Vec::new(),
            uses: Vec::new(),
            warnings: Vec::new(),
            info,
            pending: Vec::new(),
            failed_in: None,
        }
    }

    fn program(&mut self, prog: &Program) -> Result<(), Error> {
        self.collect_adts(prog)?;
        self.signatures(&prog.all_fns())?;
        self.bodies(prog)?;
        self.no_polymorphic_recursion()?;
        check_main(prog)?;
        // What's dead depends on the whole program, so only once it all checks.
        self.dead_code(prog);
        Ok(())
    }

    /// Pass 1: collects every function's signature first, so bodies can
    /// refer to functions defined later in the file (and to themselves:
    /// recursion). Nested ones too, which can be used anywhere in their
    /// block, and by each other. `fns` are all of them, with the index of the
    /// one each is nested in, if it is (see `Program::all_fns`).
    fn signatures(&mut self, fns: &[(&FnDecl, Option<usize>)]) -> Result<(), Error> {
        let mut top: Vec<FnId> = Vec::new();
        for (i, &(f, parent)) in fns.iter().enumerate() {
            let parent = parent.map(FnId);
            if parent.is_none() && f.name.name == "main" && !f.type_params.is_empty() {
                return Err(Error::ty(
                    f.type_params[0].span,
                    "`main` must not have type parameters: nothing could say what they are",
                ));
            }
            let params = generics::declare(&f.type_params)?;
            let param_tys = f
                .params
                .iter()
                .map(|p| self.resolve_with(&p.ty, &params))
                .collect::<Result<Vec<_>, _>>()?;
            let ret = self.resolve_with(&f.ret, &params)?;
            let ty = Type::func(param_tys, ret);
            let name = &f.name.name;
            let symbol = self.fn_symbols[i].clone();
            match parent {
                None => {
                    if BUILTINS.contains(&name.as_str())
                        || top.iter().any(|id| self.fns[id.0].name.name == *name)
                    {
                        return Err(Error::ty(
                            f.name.span,
                            format!("`{name}` is already defined"),
                        ));
                    }
                    top.push(FnId(i));
                }
                Some(_) => {
                    self.nested_ids.insert(f.name.span, FnId(i));
                }
            }
            self.info.symbols.insert(f.name.span, symbol.clone());
            self.fns.push(FnDef {
                name: f.name.clone(),
                symbol,
                scheme: Scheme { params, ty },
                parent,
                body: f.body.span,
            });
        }
        self.scopes.push(FnScope { at: 0, fns: top });
        self.uses = (0..fns.len()).map(|_| Uses::default()).collect();
        Ok(())
    }

    /// Pass 2: checks each function's body against its declared return type.
    fn bodies(&mut self, prog: &Program) -> Result<(), Error> {
        for (i, f) in prog.fns.iter().enumerate() {
            let notes = self.info.notes.len();
            let result = self.body(f, FnId(i));
            self.finish_notes(notes);
            if result.is_err() {
                self.failed_in = Some(f.name.name.clone());
            }
            result?;
        }
        Ok(())
    }

    /// Checks the body of `f`, which has id `id`, top-level or nested.
    fn body(&mut self, f: &FnDecl, id: FnId) -> Result<(), Error> {
        self.current = Some(id);
        let scheme = self.fns[id.0].scheme.clone();
        let Type::Fn(sig) = &scheme.ty else {
            unreachable!("a function has a function type")
        };
        let kind = NoteKind::Fn(f.name.name.clone());
        self.note(
            f.name.span,
            kind,
            Some(scheme.ty.clone()),
            Some(f.name.span),
        );
        // Inside the body its type parameters are in scope, and opaque:
        // `Type::Param` is equal only to itself.
        self.type_params = scheme.params.clone();
        let scope = self.locals.len();
        for (i, p) in f.params.iter().enumerate() {
            if f.params[..i].iter().any(|q| q.name.name == p.name.name) {
                return Err(Error::ty(
                    p.name.span,
                    format!("parameter `{}` is declared twice", p.name.name),
                ));
            }
            let ty = sig.params[i].clone();
            self.mention(&ty);
            let local = Local::param(&p.name.name, ty, p.name.span);
            self.note_binds(std::slice::from_ref(&local), false);
            self.locals.push(local);
        }
        let ret = (*sig.ret).clone();
        self.mention(&ret);
        let vars = self.vars.len();
        self.check(&f.body, &ret)?;
        self.leaves(&f.body, &ret, Exit::Return);
        self.end_scope(scope);
        self.solve_all(vars)
    }

    /// Pass 0: builds the table of structs and enums, local ones too, and
    /// fills in `arities`. Rejects duplicate names, fields, variants and
    /// type parameters, type parameters no field uses, types that contain
    /// themselves, and polymorphic recursion.
    fn collect_adts(&mut self, prog: &Program) -> Result<(), Error> {
        let decls = prog.all_types();
        // All names first, so declarations can mention each other in any
        // order.
        let mut declared = HashSet::new();
        for ((d, _), def) in decls.iter().zip(&self.types) {
            let name = d.name();
            if matches!(name.name.as_str(), "int" | "bool") {
                return Err(Error::ty(
                    name.span,
                    format!("`{}` is a built-in type", name.name),
                ));
            }
            if !declared.insert((def.block, &name.name)) {
                return Err(Error::ty(
                    name.span,
                    format!("type `{}` is already defined", name.name),
                ));
            }
            let arity = d.type_params().len();
            self.arities.insert(def.symbol.clone(), arity);
        }

        let mut adts = Adts::new();
        // Every field's type, with the symbol of the type it's in and where
        // it was written, for the recursion checks.
        let mut field_types = Vec::new();
        for ((d, _), def) in decls.iter().zip(&self.types) {
            let params = generics::declare(d.type_params())?;
            let kind = match d {
                TypeDecl::Struct(s) => {
                    let mut fields: Vec<(String, Type)> = Vec::new();
                    for f in &s.fields {
                        if fields.iter().any(|(n, _)| *n == f.name.name) {
                            return Err(Error::ty(
                                f.name.span,
                                format!("field `{}` is declared twice", f.name.name),
                            ));
                        }
                        let t = self.resolve_with(&f.ty, &params)?;
                        field_types.push((def.symbol.as_str(), f.ty.span, t.clone()));
                        fields.push((f.name.name.clone(), t));
                    }
                    AdtKind::Struct(fields)
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
                            let t = self.resolve_with(f, &params)?;
                            field_types.push((def.symbol.as_str(), f.span, t.clone()));
                            tys.push(t);
                        }
                        variants.push((v.name.name.clone(), tys));
                    }
                    AdtKind::Enum(variants)
                }
            };
            adts.insert(def.symbol.clone(), AdtDef { params, kind });
        }

        // `struct S<T> { f: fn(S<Option<T>>) -> int }` would need `S<int>`,
        // so `S<Option<int>>`, so `S<Option<Option<int>>>`...
        let mut uses = Vec::new();
        for (owner, span, t) in &field_types {
            each_adt(t, &mut |name, args| {
                uses.push(Use {
                    from: owner,
                    to: name,
                    args,
                    span: *span,
                })
            });
        }
        let params: HashMap<&str, &[String]> = adts
            .iter()
            .map(|(name, def)| (name.as_str(), def.params.as_slice()))
            .collect();
        generics::polymorphic_recursion(&uses, &params)?;

        // A struct that holds itself, even via other types, would need
        // infinite memory: `struct A { a: A }` has the size of an int
        // plus... itself. Making that work takes a pointer, and a story for
        // who frees it: that's step 5, ownership.
        for (owner, span, t) in field_types {
            if contains(&adts, &t, owner, &mut HashSet::new()) {
                return Err(Error::ty(
                    span,
                    format!(
                        "`{}` contains itself, so it would have infinite size \
                         (holding itself through a pointer comes in step 5)",
                        source_name(owner)
                    ),
                ));
            }
        }
        self.adts = adts;
        Ok(())
    }

    /// What type name `name`, written at `span` where type parameters
    /// `params` are in scope, refers to, if it's a struct or enum or an
    /// outer type parameter (see `names.rs`).
    fn type_name(&self, name: &str, span: Span, params: &[String]) -> Option<TypeName> {
        let Some(names) = &self.type_names else {
            if params.iter().any(|p| p == name) {
                return None;
            }
            let arity = *self.arities.get(name)?;
            return Some(TypeName::Adt(name.to_string(), arity));
        };
        match names.get(&span)? {
            Scoped::Adt(symbol) => {
                let arity = *self.arities.get(symbol)?;
                Some(TypeName::Adt(symbol.clone(), arity))
            }
            Scoped::OuterParam(keyword, item) => Some(TypeName::OuterParam(keyword, item.clone())),
        }
    }

    /// The symbol of the struct or enum that `name`, written at `span`,
    /// refers to, if it's one.
    fn adt_symbol<'a>(&'a self, name: &'a str, span: Span) -> Option<&'a str> {
        match &self.type_names {
            None => self.arities.contains_key(name).then_some(name),
            Some(names) => match names.get(&span)? {
                Scoped::Adt(symbol) => Some(symbol),
                Scoped::OuterParam(..) => None,
            },
        }
    }

    /// A type written where type parameters `params` are in scope.
    fn resolve_with(&self, t: &TypeExpr, params: &[String]) -> Result<Type, Error> {
        resolve(t, &|name, span| self.type_name(name, span, params), params)
    }

    /// A type written inside the function being checked.
    fn resolve(&self, t: &TypeExpr) -> Result<Type, Error> {
        self.resolve_with(t, &self.type_params)
    }

    /// Rejects polymorphic recursion between functions: see
    /// `generics::polymorphic_recursion`.
    fn no_polymorphic_recursion(&self) -> Result<(), Error> {
        // Which function each use is in, from where they are.
        let mut uses = Vec::new();
        for (span, instance) in &self.info.instances {
            // The innermost function it's in.
            let from = self
                .fns
                .iter()
                .filter(|f| f.body.start <= span.start && span.end <= f.body.end)
                .min_by_key(|f| f.body.end - f.body.start);
            if let Instance::Fn(to, args) = instance
                && let Some(from) = from
            {
                uses.push(Use {
                    from: &from.symbol,
                    to,
                    args,
                    span: *span,
                });
            }
        }
        // In source order, so the first one is reported.
        uses.sort_by_key(|u| u.span.start);
        let params: HashMap<&str, &[String]> = self
            .fns
            .iter()
            .map(|f| (f.symbol.as_str(), f.scheme.params.as_slice()))
            .collect();
        generics::polymorphic_recursion(&uses, &params)
    }

    /// Finds a variable's type, and notes that it was used and what it
    /// refers to. A generic function is instantiated: its type parameters
    /// are replaced by the type arguments `targs`, if any were written, or
    /// else by a fresh variable each. So each use is separate, and `id(1)`
    /// and `id(true)` can both be fine.
    fn lookup(
        &mut self,
        name: &str,
        targs: &[TypeExpr],
        span: Span,
    ) -> Result<Option<Type>, Error> {
        // Notes are for the name, not any `::<..>` after it.
        let at = Span::new(span.start, span.start + name.len());
        let (scheme, def, symbol) = match self.binding(name) {
            None => return Ok(None),
            Some(Binding::Local(i)) => {
                if i < self.barrier {
                    return Err(self.capture(i, span));
                }
                if !targs.is_empty() {
                    return Err(Error::ty(
                        span,
                        format!(
                            "`{name}` is a variable, and only functions, structs and enums take \
                             type arguments"
                        ),
                    ));
                }
                let local = &mut self.locals[i];
                local.used = true;
                let (ty, def) = (local.ty.clone(), local.sites[0].span);
                self.note(
                    at,
                    NoteKind::Var(name.to_string()),
                    Some(ty.clone()),
                    Some(def),
                );
                return Ok(Some(ty));
            }
            Some(Binding::Fn(id)) => {
                self.uses().fns.insert(id);
                let f = &self.fns[id.0];
                (f.scheme.clone(), Some(f.name.span), Some(f.symbol.clone()))
            }
            Some(Binding::Builtin(b)) => {
                let ty = builtin_type(b);
                let scheme = Scheme {
                    params: Vec::new(),
                    ty,
                };
                (scheme, None, None)
            }
        };
        let args = self.type_args(name, &scheme.params, targs, span, "(..)")?;
        let ty = Subst::new(&scheme.params, &args).apply(&scheme.ty);
        let instance = symbol.map(|symbol| {
            self.info.symbols.insert(span, symbol.clone());
            (span, Instance::Fn(symbol, args))
        });
        let kind = NoteKind::Fn(name.to_string());
        self.note_use(at, kind, ty.clone(), def, instance, targs.is_empty());
        Ok(Some(ty))
    }

    /// The type arguments for a use at `span` of `name`, which has type
    /// parameters `params`: those written in `targs`, or a fresh variable
    /// for each if none were. `rest` is what follows `name::<..>` in an
    /// example of writing them, for when they can't be inferred.
    fn type_args(
        &mut self,
        name: &str,
        params: &[String],
        targs: &[TypeExpr],
        span: Span,
        rest: &str,
    ) -> Result<Vec<Type>, Error> {
        if targs.is_empty() {
            let example = vec!["int"; params.len()].join(", ");
            return Ok(params
                .iter()
                .map(|p| {
                    self.fresh(
                        span,
                        format!("`{p}` for this use of `{name}`"),
                        format!(
                            "nothing here says what it is, so write it, as in \
                             `{name}::<{example}>{rest}`, or annotate where the result goes"
                        ),
                    )
                })
                .collect());
        }
        if targs.len() != params.len() {
            return Err(generics::arity_error(
                &format!("`{name}`"),
                params.len(),
                targs.len(),
                span,
            ));
        }
        targs.iter().map(|t| self.type_arg(t)).collect()
    }

    /// A type argument written in `::<..>`, where `_` may stand for any part
    /// of it, to be inferred: `pair::<int, _>`, `id::<Option<_>>`.
    ///
    /// `resolve` can't make type variables, so each `_` is renamed to a
    /// type parameter no source program can name, resolved like any other,
    /// and then replaced by a fresh variable.
    fn type_arg(&mut self, t: &TypeExpr) -> Result<Type, Error> {
        let mut holes = Vec::new();
        let t = generics::name_holes(t, &mut holes);
        let mut params = self.type_params.clone();
        params.extend(holes.iter().map(|(name, _)| name.clone()));
        let ty = self.resolve_with(&t, &params)?;
        let (names, vars): (Vec<String>, Vec<Type>) = holes
            .into_iter()
            .map(|(name, span)| {
                let what = "the type `_` stands for here".to_string();
                let var = self.fresh(span, what, "write it out instead".into());
                (name, var)
            })
            .unzip();
        Ok(Subst::new(&names, &vars).apply(&ty))
    }

    /// What the function being checked refers to.
    fn uses(&mut self) -> &mut Uses {
        let f = self.current.expect("checking outside a function");
        &mut self.uses[f.0]
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
                    Type::Fn(sig) if sig.params.len() == l.params.len() => {
                        Some((sig.params, *sig.ret))
                    }
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
            //
            // If x is a generic function, T is its type with the type
            // arguments substituted for its type parameters: the ones written
            // in `x::<..>`, or else a fresh variable for each (see `lookup`).
            ExprKind::Var(name, targs) => self
                .lookup(name, targs, e.span)?
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

                //   Γ ⊢ l : T   Γ ⊢ r : T   T contains no function type or type parameter
                //  ------------------------------------------------------------------ (T-Eq) for == !=
                //                         Γ ⊢ l == r : bool
                //
                // See `comparable` for why those are excluded. If T isn't
                // fully known yet, as in `|a, b| a == b`, this premise waits
                // until it is, at the end of the function.
                BinOp::Eq | BinOp::Ne => {
                    let t = self.synth(l)?;
                    self.check(r, &t)?;
                    let t = self.solved(&t);
                    if unify::has_vars(&t) {
                        self.comparisons.push((t, e.span, *op));
                    } else {
                        self.comparable(&t, e.span, *op)?;
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
                    Type::Fn(sig) => (sig.params, *sig.ret),
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
                        let sig = Type::func(params.clone(), ret.clone());
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
            //
            // For a generic struct, S is `S<A1, .., Ak>`, with the type
            // arguments written or a fresh variable for each type parameter,
            // and T1..Tn mention them.
            ExprKind::StructLit(name, targs, inits) => {
                let ty = self.struct_type(name, targs)?;
                let Type::Adt(symbol, _) = &ty else {
                    unreachable!("a struct")
                };
                let symbol = symbol.clone();
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
                    self.note_field(&symbol, &init.name, &fields[i].1);
                    self.check(&init.value, &fields[i].1)?;
                    self.leaves(&init.value, &fields[i].1, Exit::Store);
                }
                if let Some(i) = given.iter().position(|g| !g) {
                    return Err(Error::ty(
                        e.span,
                        format!("missing field `{}` in `{}`", fields[i].0, name.name),
                    ));
                }
                self.uses().types.insert(symbol.clone());
                self.uses().structs.insert(symbol);
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
                    Some((_, ft)) => {
                        if let Type::Adt(s, _) = &t {
                            self.note_field(&s.clone(), field, &ft);
                        }
                        Ok(ft)
                    }
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
            ExprKind::Variant(ty, targs, variant, args) => {
                let (t, i) = self.variant(ty, targs, variant)?;
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
                let Type::Adt(symbol, _) = &t else {
                    unreachable!("an enum")
                };
                let (ty, variant) = (symbol.clone(), variant.name.clone());
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
            let local = Local::param(name, t, p.name.span);
            self.note_binds(std::slice::from_ref(&local), p.ty.is_none());
            self.locals.push(local);
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
        Ok(Type::func(params, ret))
    }

    /// The type a closure's annotation names, which must fit the one
    /// `wanted` from the context, if any.
    fn annotation(&mut self, ann: &TypeExpr, wanted: Option<Type>) -> Result<Type, Error> {
        let t = self.resolve(ann)?;
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
    //
    // A nested `fn` extends the context of the whole block, before it as
    // well as after, so that it can be recursive, and nested functions can
    // call each other. Its body is checked as if it were at the top level.
    fn block(
        &mut self,
        stmts: &[Stmt],
        tail: Option<&Expr>,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        let scope = self.locals.len();
        let fns: Vec<&FnDecl> = stmts.iter().filter_map(Stmt::as_fn).collect();
        for (i, f) in fns.iter().enumerate() {
            if fns[..i].iter().any(|g| g.name.name == f.name.name) {
                return Err(Error::ty(
                    f.name.span,
                    format!("`{}` is already defined", f.name.name),
                ));
            }
        }
        if !fns.is_empty() {
            let fns = fns.iter().map(|f| self.nested_ids[&f.name.span]).collect();
            self.scopes.push(FnScope { at: scope, fns });
        }
        for (k, stmt) in stmts.iter().enumerate() {
            match stmt {
                Stmt::Fn(f) => {
                    let fix = to_closure(f, stmts, k, tail);
                    self.nested(f, fix)?;
                }
                // Checked with the others, before any function.
                Stmt::Type(_) => {}
                Stmt::Let { pat, ty, init } => {
                    let t = match ty {
                        Some(ann) => {
                            let t = self.resolve(ann)?;
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
                    self.note_binds(&binds, ty.is_none());
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
        if !fns.is_empty() {
            self.scopes.pop();
        }
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
            self.note_binds(&b, true);
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
            PatternKind::Variant(name, targs, variant, args) => {
                let (t, i) = self.variant(name, targs, variant)?;
                let fields = variant_fields(&self.adts, &t, i);
                if let Type::Adt(symbol, _) = &t {
                    self.uses().types.insert(symbol.clone());
                }
                self.pattern_is(p, ty, t)?;
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
            PatternKind::Struct(name, targs, field_pats, rest) => {
                let t = self.struct_type(name, targs)?;
                let fields = struct_fields(&self.adts, &t).expect("a struct");
                let Type::Adt(symbol, _) = &t else {
                    unreachable!("a struct")
                };
                let symbol = symbol.clone();
                self.uses().types.insert(symbol.clone());
                self.pattern_is(p, ty, t)?;
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
                    self.note_field(&symbol, &fp.name, &fields[i].1);
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
    /// If it's generic, each type parameter gets a fresh variable, to be
    /// worked out from the fields or from where it's used.
    fn struct_type(&mut self, name: &Ident, targs: &[TypeExpr]) -> Result<Type, Error> {
        let def = self
            .adt_symbol(&name.name, name.span)
            .and_then(|s| self.adts.get(s));
        match def.map(|d| &d.kind) {
            Some(AdtKind::Struct(_)) => self.instantiate(name, targs, " { .. }", NoteKind::Struct),
            Some(AdtKind::Enum(_)) => Err(Error::ty(
                name.span,
                format!("`{}` is an enum, not a struct", name.name),
            )),
            None => Err(Error::ty(
                name.span,
                format!("unknown type `{}`", name.name),
            )),
        }
    }

    /// `name<A1, .., Ak>`, for struct or enum `name`, which must be one,
    /// with the type arguments `targs` or else a fresh variable for each
    /// type parameter, noted as a use of it at `name`'s span. `rest`
    /// follows `name::<..>` in an example of writing them.
    fn instantiate(
        &mut self,
        name: &Ident,
        targs: &[TypeExpr],
        rest: &str,
        kind: fn(String) -> NoteKind,
    ) -> Result<Type, Error> {
        let symbol = self
            .adt_symbol(&name.name, name.span)
            .expect("a struct or enum");
        let symbol = symbol.to_string();
        let params = self.adts[&symbol].params.clone();
        let args = self.type_args(&name.name, &params, targs, name.span, rest)?;
        let ty = Type::Adt(symbol.clone(), args.clone());
        let def = self.defs.types.get(&symbol).copied();
        let instance = Instance::Adt(symbol, args);
        let kind = kind(name.name.clone());
        self.note_use(
            name.span,
            kind,
            ty.clone(),
            def,
            Some((name.span, instance)),
            targs.is_empty(),
        );
        Ok(ty)
    }

    /// Looks up `E::V`, returning the enum's type, instantiated as in
    /// `instantiate`, and the variant's index.
    fn variant(
        &mut self,
        ty: &Ident,
        targs: &[TypeExpr],
        variant: &Ident,
    ) -> Result<(Type, usize), Error> {
        let symbol = self.adt_symbol(&ty.name, ty.span).map(str::to_string);
        let def = symbol.as_ref().and_then(|s| self.adts.get(s));
        let variants = match def.map(|d| &d.kind) {
            Some(AdtKind::Enum(variants)) => variants,
            Some(AdtKind::Struct(_)) => {
                return Err(Error::ty(
                    ty.span,
                    format!("`{}` is a struct, not an enum", ty.name),
                ));
            }
            None => return Err(Error::ty(ty.span, format!("unknown type `{}`", ty.name))),
        };
        let Some(i) = variants.iter().position(|(v, _)| *v == variant.name) else {
            return Err(Error::ty(
                variant.span,
                format!("`{}` has no variant `{}`", ty.name, variant.name),
            ));
        };
        let rest = format!("::{}", variant.name);
        let t = self.instantiate(ty, targs, &rest, NoteKind::Enum)?;
        let fields = variant_fields(&self.adts, &t, i);
        let as_fn = match fields.is_empty() {
            true => t.clone(),
            false => Type::func(fields, t.clone()),
        };
        let key = (symbol.expect("an enum"), variant.name.clone());
        let def = self.defs.variants.get(&key).copied();
        let kind = NoteKind::Variant(ty.name.clone(), variant.name.clone());
        self.note(variant.span, kind, Some(as_fn), def);
        Ok((t, i))
    }

    /// Notes a use of field `field` of struct `s`, of type `ty`.
    fn note_field(&mut self, s: &str, field: &Ident, ty: &Type) {
        let def = self.defs.fields.get(&(s.to_string(), field.name.clone()));
        let kind = NoteKind::Field(field.name.clone());
        self.note(field.span, kind, Some(ty.clone()), def.copied());
    }

    /// The rest of T-Eq's premise: `t`, which is known, contains no
    /// function type or type parameter.
    ///
    /// Equality on functions is excluded: deciding whether two functions
    /// behave the same on every input is undecidable, and comparing them by
    /// address would make `==` depend on how the compiler happened to lay out
    /// code. Structs and enums compare field by field, so they're excluded if
    /// any field is (or holds) a function.
    ///
    /// A type parameter `T` could be any type, functions included, so `==`
    /// on a `T` can't be allowed either. Saying "any type that has `==`"
    /// takes a bound, `T: Eq`, which comes with traits (step 6).
    fn comparable(&self, t: &Type, span: Span, op: BinOp) -> Result<(), Error> {
        let message = match (t, self.incomparable(t)) {
            (_, None) => return Ok(()),
            (Type::Fn(..), _) => format!("cannot compare functions with `{op}`"),
            (Type::Param(p), _) => format!(
                "cannot compare values of type `{p}` with `{op}`: `{p}` is a type \
                 parameter, so it may be a function; that takes a bound like `{p}: Eq`, \
                 which comes with traits (step 6)"
            ),
            (_, Some(Type::Param(p))) => format!(
                "cannot compare `{t}` with `{op}`: it contains `{p}`, a type parameter, \
                 which may be a function"
            ),
            (_, Some(_)) => format!("cannot compare `{t}` with `{op}`: it contains a function"),
        };
        Err(Error::ty(span, message))
    }

    /// The first function type or type parameter in `t` or, if it's a
    /// struct or enum, in its fields. This terminates because types can't
    /// contain themselves.
    fn incomparable(&self, t: &Type) -> Option<Type> {
        match t {
            Type::Fn(..) | Type::Param(_) => Some(t.clone()),
            Type::Adt(..) => all_fields(&self.adts, t)
                .iter()
                .find_map(|t| self.incomparable(t)),
            // Only called on types with no variables in them.
            Type::Int | Type::Bool | Type::Unit | Type::Var(_) => None,
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
