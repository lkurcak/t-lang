//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `int`, `bool`, `()` and function types, plus `let`, `if` and
//! recursive functions (step 1), plus structs, enums and `match`
//! (step 2, see docs/02-adts.md), plus closures, nested functions and types,
//! and type inference (step 3, see docs/03-inference.md), plus generics
//! (step 4, see docs/04-generics.md), plus references, boxes, mutation and
//! moves (step 5, see docs/05-ownership.md), plus `return`, `break`,
//! `continue`, `loop` and the never type `!` (see
//! docs/05.1-control-flow.md), plus traits, impls and methods (step 6,
//! see docs/06-traits.md, and `check/traits.rs`), plus opaque types, `impl
//! Fn(int) -> int` in a return type (see docs/06c.1-opaque-types.md).
//! Whether a program uses its
//! references and moved values correctly is checked after its types are,
//! by the borrow checker (`borrowck`), from what `typed` records here.
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

mod closures;
mod generics;
mod info;
mod lints;
mod names;
mod scope;
mod traits;
pub(crate) mod typed;
mod unify;

pub(crate) use generics::{Lifetimes, TypeName, resolve};
pub use info::{Info, Instance, Note, NoteKind};
pub use scope::source_name;

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span, Warning};
use crate::exhaust::{self, Ctor, Pat, Redundant};
use crate::traits::{Pred, Traits};
use crate::types::{
    AdtDef, AdtKind, Adts, Region, Scheme, Subst, Type, all_fields, struct_fields, variant_fields,
};
use generics::Use;
use info::Defs;
use info::Resolved;
use lints::{
    Uses, mergeable, narrowable, unreachable_alt, unreachable_arm, unreachable_code, unused,
    unused_doc, unused_label,
};
use names::{Scoped, TypeDef};
use scope::{Binding, FnDef, FnId, FnScope, to_closure};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use typed::{Bind, Res, Typed};
use unify::Var;

/// Built-in functions, available everywhere. Shared with the interpreter.
pub const BUILTINS: &[&str] = &["print", "print_bool", "replace", "swap"];

/// The type of built-in function `name`.
pub(crate) fn builtin_scheme(name: &str) -> Scheme {
    match name {
        // print : fn(int) -> ()
        "print" => Scheme::mono(Type::func(vec![Type::Int], Type::Unit)),
        // print_bool : fn(bool) -> ()
        "print_bool" => Scheme::mono(Type::func(vec![Type::Bool], Type::Unit)),
        // replace : ∀'a T. fn(&'a mut T, T) -> T
        //
        // Puts the new value where the reference points, and gives back the
        // old one, so nothing is thrown away: how to change what's behind a
        // `&mut` whose type isn't `Discard`.
        "replace" => {
            let t = Type::Param("T".into());
            let r = Type::Ref(Region::Named("'a".into()), true, Box::new(t.clone()));
            Scheme {
                params: vec!["T".into()],
                lifetimes: vec!["'a".into()],
                bounds: Vec::new(),
                preds: Vec::new(),
                eqs: Vec::new(),
                ty: Type::func(vec![r, t.clone()], t),
            }
        }
        // swap : ∀'a 'b T. fn(&'a mut T, &'b mut T) -> ()
        //
        // Exchanges two values, making and throwing away none: the more
        // basic of the two, as `replace` can be written with it and not the
        // other way round, having no `T` of its own to put in.
        "swap" => {
            let t = Type::Param("T".into());
            let r = |l: &str| Type::Ref(Region::Named(l.into()), true, Box::new(t.clone()));
            Scheme {
                params: vec!["T".into()],
                lifetimes: vec!["'a".into(), "'b".into()],
                bounds: Vec::new(),
                preds: Vec::new(),
                eqs: Vec::new(),
                ty: Type::func(vec![r("'a"), r("'b")], Type::Unit),
            }
        }
        _ => unreachable!("unknown builtin `{name}`"),
    }
}

/// Type-checks `prog`, adding any warnings to `warnings`, in source order.
/// Those found before an error are kept, and so is what was learned about
/// the program up to it.
pub fn check(prog: &Program, warnings: &mut Vec<Warning>) -> (Info, Result<(), Error>) {
    let mut cx = Checker::new(prog);
    cx.warnings.extend(prog.stray_docs.iter().map(unused_doc));
    let result = cx.program(prog);
    cx.type_notes(prog);
    cx.info.types = cx.defs.types.clone();
    cx.info.projections = cx.proj_traits.take();
    cx.warnings.sort_by_key(|w| w.span.start);
    warnings.append(&mut cx.warnings);
    (cx.info, result)
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
    if let Type::Adt(name, _, args) = t {
        f(name, args);
    }
    for p in t.parts() {
        each_adt(p, f);
    }
}

/// Does a value of type `ty` hold a `target`, at any depth? Function types
/// don't count: a function value is a code address, not the values its
/// signature mentions. Nor do references and boxes, which hold an address.
/// `struct A { b: Wrap<A> }` counts, through `Wrap`'s field of type `T`.
///
/// This terminates because polymorphic recursion is ruled out first, so only
/// finitely many types can turn up in here.
fn contains(adts: &Adts, ty: &Type, target: &str, seen: &mut HashSet<Type>) -> bool {
    let Type::Adt(name, ..) = ty else {
        return false;
    };
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

/// Every `impl Tr` written in return type `t`, where they can be: not in a
/// function type, which would be a different type for each function, nor
/// in another's bounds, nor a projection.
fn opaques_in<'t>(t: &'t TypeExpr, out: &mut Vec<&'t TypeExpr>) {
    match &t.kind {
        TypeExprKind::Impl(_) => out.push(t),
        TypeExprKind::Named(_, args) => args.iter().for_each(|a| opaques_in(a, out)),
        TypeExprKind::Ref(_, _, inner) => opaques_in(inner, out),
        TypeExprKind::Fn(..)
        | TypeExprKind::Proj(..)
        | TypeExprKind::Unit
        | TypeExprKind::Never
        | TypeExprKind::Lifetime(_) => {}
    }
}

/// The symbols of the opaque types in `t`.
fn opaques_of(t: &Type) -> Vec<String> {
    let mut out = Vec::new();
    if let Type::Opaque(s, ..) = t {
        out.push(s.clone());
    }
    for p in t.parts() {
        out.extend(opaques_of(p));
    }
    out
}

/// `t`, with every `from` in it replaced by `to`.
fn replace_type(t: &Type, from: &Type, to: &Type) -> Type {
    match t == from {
        true => to.clone(),
        false => t.map(|p| replace_type(p, from, to)),
    }
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
    /// Declared `mut`?
    mutable: bool,
    /// From the `mut` to the name, if it's `mut`.
    mut_span: Option<Span>,
    /// Bound by reference by match ergonomics, and if so, `&mut`?
    by_ref: Option<bool>,
    /// Where it's bound: one place, or one per alternative of an or-pattern.
    sites: Vec<Site>,
    /// Whether anything has read it, for the unused-variable warning.
    used: bool,
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
            mutable: false,
            mut_span: None,
            by_ref: None,
            sites: vec![Site {
                span,
                shorthand: false,
            }],
            used: false,
        }
    }

    /// Its id: where its name is first bound (see `typed`).
    fn id(&self) -> Span {
        self.sites[0].span
    }

    /// A parameter.
    fn param(name: &str, ty: Type, span: Span) -> Local {
        Local::new(name, ty, span)
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

pub(crate) struct Checker {
    /// What every struct and enum is made of, by symbol.
    adts: Adts,
    /// How many lifetime and type arguments each struct and enum takes, by
    /// symbol.
    arities: HashMap<String, (usize, usize)>,
    /// Every struct and enum, top-level and local, in the order of
    /// `Program::all_types`.
    types: Vec<TypeDef>,
    /// What each name in a type refers to, by its span (see `names.rs`).
    type_names: HashMap<Span, Scoped>,
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
    /// Its lifetime parameters, elided ones too.
    lifetimes: Vec<String>,
    /// What the borrow checker needs to know about each body.
    typed: Typed,
    /// The closures that capture in the function being checked, whose
    /// captures are worked out at its end (see `closures`).
    closures: Vec<closures::Pending>,
    /// Why each closure implements only the `Fn` traits it does, by
    /// symbol, for messages: "it moves `b` out".
    closure_why: HashMap<String, String>,
    /// The calls in the function being checked of what isn't a function
    /// type, with the callee's type, and where: how each gets at what it
    /// calls is worked out at its end (`Checker::settle_calls`).
    calls: Vec<(*const Expr, Type, Span)>,
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
    /// Every trait and impl, the built-in ones too.
    traits: Traits,
    /// Where each impl of the program is in `traits.impls`.
    impl_ids: Vec<usize>,
    /// Each method's id, by where its name is declared.
    method_ids: HashMap<Span, FnId>,
    /// Whether each function, by `FnId`, takes `self`: a method that does.
    receivers: Vec<bool>,
    /// The bounds that hold in what's being checked: its own, its impl's,
    /// and in a trait's default method, `Self: Trait`. The *environment*
    /// obligations are proved in.
    env: Vec<Pred>,
    /// What `Self` is in the function being checked: the type of the impl
    /// it's in, or in a trait, `Self` as a type parameter.
    self_ty: Option<Type>,
    /// Obligations whose types weren't fully known yet when they came up,
    /// like `==` on a value of a type to be inferred. Proved once the
    /// function's types are.
    obligations: Vec<traits::Obligation>,
    /// Unused variables warned about, by index in `warnings`, with their
    /// types: once those are known, one that isn't `Discard` gets no fix
    /// to rename it, which wouldn't help (see `settle_unused`).
    unused: Vec<(usize, Type)>,
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
    /// What a `return` in the code being checked returns from: the type of
    /// the function's or closure's result. `None` in a closure whose result
    /// type is still to be inferred, until a `return` needs it.
    ret: Option<Type>,
    /// The loops around the code being checked, innermost last, up to the
    /// function's start: what a `break` or `continue` there goes to.
    loops: Vec<Target>,
    /// Whether the code checked so far, in the function being checked,
    /// certainly never gets to what comes next, *diverges*: it returns,
    /// breaks out, or loops forever on every path. With where. See
    /// `block`.
    diverges: Option<Span>,
    /// The types a `!` was wanted as while they were still unknown. Those
    /// that nothing else decides by the end of the function are `!`.
    diverging: Vec<Type>,
    /// The trait of each projection written, by its span, as it's worked
    /// out while resolving types (see `Info::projections`).
    proj_traits: std::cell::RefCell<HashMap<Span, String>>,
    /// Whether impls are being collected: then what a projection on a type
    /// that isn't a parameter stands for isn't known yet, nor whether the
    /// type implements the trait, which `check_impls` sees to.
    collecting: bool,
    /// Where the expression being checked is, for the goals it makes.
    here: Span,
    /// Projections on types not known yet, each waiting for its type to
    /// be, with what it must turn out to be (see `settle`).
    goals: Vec<Goal>,
    /// The symbol of each opaque type, `impl Tr` in a return type, by
    /// where it's written.
    opaque_at: HashMap<Span, String>,
    /// The function whose signature is being worked out, by symbol and
    /// name: what owns the opaque types in it.
    sig_owner: (String, String),
}

/// A projection `proj`, `<T as Tr>::Item`, whose type `T` isn't known yet,
/// so that what it stands for isn't, which must turn out to be `is`, for
/// what's at `span`. Working it out only ever goes forward, from `T` to
/// what it stands for, as different types can have the same: `T` is never
/// inferred from it (see `unify`).
#[derive(Debug, Clone)]
pub(super) struct Goal {
    proj: Type,
    is: Type,
    span: Span,
    /// What needs it to be `is`, if something said so: an equality bound
    /// of what's named this.
    who: Option<String>,
}

/// What a `break` or `continue` goes to.
#[derive(Debug)]
pub(crate) enum Target {
    Loop(Box<LoopCx>),
    /// The start of a closure's body, which is a function of its own: no
    /// `break` gets out of it.
    Closure,
}

/// A loop being checked.
#[derive(Debug)]
pub(crate) struct LoopCx {
    label: Option<Label>,
    /// Whether a `break` or `continue` has used its label.
    used: bool,
    /// A `while`, whose `break` can't give a value, rather than a `loop`.
    is_while: bool,
    /// The type of the loop's value wanted from it, when checking.
    want: Option<Type>,
    /// The type of the values its `break`s give it, once one is seen. It
    /// stays `None` if none ever gets out, so then the loop never ends, and
    /// is a `!`.
    value: Option<Type>,
}

impl Checker {
    fn new(prog: &Program) -> Checker {
        let fn_symbols = scope::symbols(prog);
        let (types, type_names) = names::names(prog, &prog.all_fns(), &fn_symbols);
        let mut info = Info::default();
        for (span, name) in &type_names {
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
            lifetimes: Vec::new(),
            typed: Typed::default(),
            closures: Vec::new(),
            closure_why: HashMap::new(),
            calls: Vec::new(),
            locals: Vec::new(),
            barrier: 0,
            closure_fix: None,
            vars: Vec::new(),
            traits: Traits::new(),
            impl_ids: Vec::new(),
            method_ids: HashMap::new(),
            receivers: Vec::new(),
            env: Vec::new(),
            self_ty: None,
            obligations: Vec::new(),
            unused: Vec::new(),
            uses: Vec::new(),
            warnings: Vec::new(),
            info,
            pending: Vec::new(),
            ret: None,
            loops: Vec::new(),
            diverges: None,
            diverging: Vec::new(),
            proj_traits: Default::default(),
            collecting: false,
            here: Span::new(0, 0),
            goals: Vec::new(),
            opaque_at: HashMap::new(),
            sig_owner: Default::default(),
        }
    }

    fn program(&mut self, prog: &Program) -> Result<(), Error> {
        self.trait_names(prog)?;
        self.collect_adts(prog)?;
        self.collect_traits(prog)?;
        let fns = prog.all_fns();
        for (i, (f, _)) in fns
            .iter()
            .enumerate()
            .skip(prog.fns.len())
            .take(prog.methods().len())
        {
            self.method_ids.insert(f.name.span, FnId(i));
        }
        self.collect_impls(prog)?;
        self.projections_known();
        self.derive(prog)?;
        self.signatures(prog, &fns)?;
        self.check_impls(prog)?;
        self.info.traits = self.traits.clone();
        self.bodies(prog)?;
        // What each closure is is known once the body it's in is, and so
        // what each opaque type stands for.
        self.info.traits.closures = self.traits.closures.clone();
        self.opaque_cycles()?;
        self.info.traits.opaques = self.traits.opaques.clone();
        self.info.opaques = self.opaque_at.clone();
        self.no_polymorphic_recursion()?;
        check_main(prog)?;
        // What's dead depends on the whole program, so only once it all checks.
        self.dead_code(prog);
        // Needs there to be no polymorphic recursion, to end.
        self.infinite_types(prog);
        Ok(())
    }

    /// Pass 1: collects every function's signature first, so bodies can
    /// refer to functions defined later in the file (and to themselves:
    /// recursion). Nested ones too, which can be used anywhere in their
    /// block, and by each other. `fns` are all of them, with the index of the
    /// one each is nested in, if it is (see `Program::all_fns`). Methods
    /// too, which are generic over their impl's type parameters as well as
    /// their own, but aren't in scope anywhere: a call names them through
    /// a type or a value.
    fn signatures(
        &mut self,
        prog: &Program,
        fns: &[(&FnDecl, Option<usize>)],
    ) -> Result<(), Error> {
        let methods = prog.methods();
        let mut top: Vec<FnId> = Vec::new();
        for (i, &(f, parent)) in fns.iter().enumerate() {
            let parent = parent.map(FnId);
            let method = i.checked_sub(prog.fns.len()).and_then(|j| methods.get(j));
            if parent.is_none()
                && method.is_none()
                && f.name.name == "main"
                && !f.generics.types.is_empty()
            {
                return Err(Error::ty(
                    f.generics.types[0].span,
                    "`main` must not have type parameters: nothing could say what they are",
                ));
            }
            self.sig_owner = (self.fn_symbols[i].clone(), f.name.name.clone());
            let scheme = match method {
                Some(&(_, MethodOf::Impl(k))) => {
                    let d = self.traits.impls[self.impl_ids[k]].clone();
                    let outer = traits::Outer {
                        params: d.params.clone(),
                        lifetimes: d.lifetimes.clone(),
                        bounds: d.bounds.clone(),
                        preds: d.preds.clone(),
                        eqs: d.eqs.clone(),
                    };
                    let this = d.trait_name.clone().map(|tr| Pred {
                        ty: Type::Param("Self".into()),
                        trait_name: tr,
                    });
                    self.method_scheme(f, &outer, this, Some(&d.self_ty))?
                }
                Some(&(_, MethodOf::Trait(k))) => {
                    let tr = &prog.traits[k].name.name;
                    let m = self.traits.method(tr, &f.name.name).expect("collected");
                    m.scheme.clone()
                }
                None => self.fn_scheme(f)?,
            };
            let name = &f.name.name;
            let symbol = self.fn_symbols[i].clone();
            self.receivers.push(f.receiver().is_some());
            match parent {
                None if method.is_some() => {}
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
                scheme,
                parent,
                body: f.body.span,
            });
        }
        self.scopes.push(FnScope { at: 0, fns: top });
        self.uses = (0..fns.len()).map(|_| Uses::default()).collect();
        Ok(())
    }

    /// The type of function `f`, which isn't a method.
    fn fn_scheme(&mut self, f: &FnDecl) -> Result<Scheme, Error> {
        let params = generics::declare(&f.generics.types)?;
        let declared = generics::declare_lifetimes(&f.generics.lifetimes)?;
        let bounds = generics::bounds(&f.generics, &declared, &params)?;
        let (preds, eqs) = self.preds(&f.generics, &params, &declared, true)?;
        self.env = preds.clone();
        self.adts.proj.local = eqs.clone();
        let (param_tys, ret, lifetimes) = self.signature(f, &params, &declared, true)?;
        self.env.clear();
        self.adts.proj.local.clear();
        Ok(Scheme {
            params,
            lifetimes,
            bounds,
            preds,
            eqs,
            ty: Type::func(param_tys, ret),
        })
    }

    /// The parameter and return types of `f`, which has type parameters
    /// `params` and declares lifetimes `declared`, and its lifetime
    /// parameters: those, and one more for each lifetime its parameters
    /// leave out.
    ///
    /// That's *lifetime elision*, as in Rust. A lifetime its return type
    /// leaves out is the one lifetime its parameters have, if they have
    /// exactly one: `fn first(p: &Pair) -> &int` returns something borrowed
    /// from `p`. Or a method's, if it takes `&self` or `&mut self`: what it
    /// returns is borrowed from that. Otherwise it has to be written.
    ///
    /// Each `impl Tr` in the return type is an opaque type of its own,
    /// generic over all of `f`'s lifetime and type parameters, elided ones
    /// too, which it's defined here as (see `opaque`). Not in a trait's
    /// methods, unless `opaques`.
    fn signature(
        &mut self,
        f: &FnDecl,
        params: &[String],
        declared: &[String],
        opaques: bool,
    ) -> Result<(Vec<Type>, Type, Vec<String>), Error> {
        let mut lts = Lifetimes::Params {
            declared,
            elided: Vec::new(),
        };
        let param_tys = f
            .params
            .iter()
            .map(|p| self.resolve_with(&p.ty, params, &mut lts))
            .collect::<Result<Vec<_>, _>>()?;
        let Lifetimes::Params { elided, .. } = lts else {
            unreachable!()
        };
        let mut inputs: Vec<&Region> = Vec::new();
        for r in param_tys.iter().flat_map(Type::regions) {
            if !inputs.contains(&r) {
                inputs.push(r);
            }
        }
        let input = match (f.receiver(), param_tys.first(), &inputs[..]) {
            (Some(_), Some(Type::Ref(r, _, _)), _) => Ok(r.clone()),
            (_, _, [r]) => Ok((*r).clone()),
            _ => Err(inputs.len()),
        };
        let lifetimes: Vec<String> = declared.iter().cloned().chain(elided).collect();
        let mut written = Vec::new();
        opaques_in(&f.ret, &mut written);
        if let Some(t) = written.first()
            && !opaques
        {
            return Err(Error::ty(
                t.span,
                "`impl` types can't be in a trait's methods yet: each impl would pick its own, \
                 as for an associated type",
            ));
        }
        let own: Vec<String> = params.iter().filter(|p| *p != "Self").cloned().collect();
        for t in written {
            self.opaque(t, &own, &lifetimes)?;
        }
        let mut lts = Lifetimes::Return { declared, input };
        let ret = self.resolve_with(&f.ret, params, &mut lts)?;
        Ok((param_tys, ret, lifetimes))
    }

    /// Defines the opaque type `t`, an `impl Tr` written in the return type
    /// of the function whose signature is being worked out, which has type
    /// parameters `params` and lifetimes `lifetimes`. Its bounds are those
    /// of a type parameter, with it for the parameter.
    fn opaque(
        &mut self,
        t: &TypeExpr,
        params: &[String],
        lifetimes: &[String],
    ) -> Result<(), Error> {
        let TypeExprKind::Impl(bounds) = &t.kind else {
            unreachable!("an opaque type is written `impl`")
        };
        let symbol = crate::types::opaque_symbol(&t.to_string(), t.span.start);
        let subject = Ident {
            name: symbol.clone(),
            span: t.span,
        };
        let g = Generics {
            bounds: vec![(subject, bounds.clone())],
            ..Generics::default()
        };
        let mut with_it = params.to_vec();
        with_it.push(symbol.clone());
        let (preds, eqs) = self.preds(&g, &with_it, lifetimes, true)?;
        let (owner, owner_name) = self.sig_owner.clone();
        let mut def = crate::traits::OpaqueDef {
            owner,
            owner_name,
            params: params.to_vec(),
            lifetimes: lifetimes.to_vec(),
            preds: Vec::new(),
            eqs: Vec::new(),
            hidden: None,
            span: t.span,
        };
        let s = Subst::new(std::slice::from_ref(&symbol), &[def.generic(&symbol)]);
        def.preds = preds
            .into_iter()
            .map(|p| Pred {
                ty: s.apply(&p.ty),
                trait_name: p.trait_name,
            })
            .collect();
        def.eqs = eqs
            .into_iter()
            .map(|(l, r)| (s.apply(&l), s.apply(&r)))
            .collect();
        self.opaque_at.insert(t.span, symbol.clone());
        self.adts.proj.opaques.insert(symbol.clone(), def.clone());
        self.traits.opaques.insert(symbol, def);
        Ok(())
    }

    /// The symbols of the opaque types a value of type `t` may hold: in
    /// it, and in the types of what the closures in it capture.
    fn opaques_held(&self, t: &Type) -> Vec<String> {
        let mut out = opaques_of(t);
        let mut seen = HashSet::new();
        let mut todo = vec![t.clone()];
        while let Some(t) = todo.pop() {
            each_adt(&t, &mut |name, _| {
                if let Some(c) = self.traits.closures.get(name)
                    && seen.insert(name.to_string())
                {
                    for (_, f) in &c.fields {
                        out.extend(opaques_of(f));
                        todo.push(f.clone());
                    }
                }
            });
        }
        out
    }

    /// Rejects opaque types that stand for types that hold them, through
    /// others or not: `fn f() -> impl Fn() -> int` whose closure holds
    /// what `f()` returns would be a type that holds itself, which no
    /// finite type is. Then replacing each with what it stands for, as
    /// monomorphisation does (`Projections::reveal`), ends.
    fn opaque_cycles(&self) -> Result<(), Error> {
        let mut symbols: Vec<&String> = self.traits.opaques.keys().collect();
        symbols.sort_by_key(|s| self.traits.opaques[*s].span.start);
        for start in symbols {
            let mut todo = vec![start.clone()];
            let mut seen = HashSet::new();
            while let Some(s) = todo.pop() {
                let Some(hidden) = &self.traits.opaques[&s].hidden else {
                    continue;
                };
                for o in self.opaques_held(hidden) {
                    if o == *start {
                        let d = &self.traits.opaques[start];
                        return Err(Error::ty(
                            d.span,
                            format!(
                                "the type `{}` returns would contain itself: what it returns \
                                 holds what {} returns",
                                d.owner_name,
                                match s == *start {
                                    true => "it".to_string(),
                                    false => format!("`{}`", self.traits.opaques[&s].owner_name),
                                }
                            ),
                        ));
                    }
                    if seen.insert(o.clone()) {
                        todo.push(o);
                    }
                }
            }
        }
        Ok(())
    }

    /// Pass 5: checks each function's body against its declared return
    /// type. The methods' too, which come right after the top-level
    /// functions in `Program::all_fns`.
    fn bodies(&mut self, prog: &Program) -> Result<(), Error> {
        let methods = prog.methods();
        let fns = prog.fns.iter().chain(methods.iter().map(|&(f, _)| f));
        for (i, f) in fns.enumerate() {
            self.self_ty = match methods.get(i.wrapping_sub(prog.fns.len())) {
                Some(&(_, MethodOf::Impl(k))) => {
                    Some(self.traits.impls[self.impl_ids[k]].self_ty.clone())
                }
                _ => None,
            };
            let notes = self.info.notes.len();
            let result = self.body(f, FnId(i));
            self.finish_notes(notes);
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
        let def = self.method_decl(id).unwrap_or(Some(f.name.span));
        self.note(f.name.span, kind, Some(scheme.ty.clone()), def);
        // Inside the body its type parameters are in scope, and opaque:
        // `Type::Param` is equal only to itself. Its bounds hold.
        self.type_params = scheme.params.clone();
        self.lifetimes = scheme.lifetimes.clone();
        self.env = scheme.preds.clone();
        self.adts.proj.local = scheme.eqs.clone();
        self.goals.clear();
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
            self.well_formed(&ty, p.ty.span)?;
            let mut local = Local::param(&p.name.name, ty, p.name.span);
            local.mutable = p.mutable.is_some();
            local.mut_span = p.mutable.map(|m| Span::new(m.start, p.name.span.start));
            self.note_binds(std::slice::from_ref(&local), false);
            self.bind(&local);
            self.locals.push(local);
        }
        let ret = (*sig.ret).clone();
        self.mention(&ret);
        self.well_formed(&ret, f.ret.span)?;
        let vars = self.vars.len();
        // In its own body, each of its opaque types is the type it stands
        // for, to be inferred, which must have what its bounds say.
        let mut hidden = Vec::new();
        let symbol = self.fns[id.0].symbol.clone();
        let ret = self.hide(&ret, &symbol, &f.name.name, &mut hidden)?;
        self.ret = Some(ret.clone());
        self.loops.clear();
        self.diverges = None;
        self.check(&f.body, &ret)?;
        self.end_scope(scope);
        self.solve_all(vars)?;
        for (s, var) in &hidden {
            let t = self.solution(var);
            let span = self.traits.opaques[s].span;
            if t == Type::Never {
                return Err(Error::ty(
                    span,
                    format!(
                        "can't tell what type `{}` returns: it never returns, so nothing \
                             says which type this is",
                        f.name.name
                    ),
                ));
            }
            self.traits.opaques.get_mut(s).expect("defined").hidden = Some(t.clone());
            self.adts.proj.opaques.get_mut(s).expect("defined").hidden = Some(t);
        }
        let warnings = crate::borrowck::check(self, f, &scheme)?;
        self.warnings.extend(warnings);
        Ok(())
    }

    /// `ret`, the return type of function `symbol`, called `name`, with
    /// each of its opaque types replaced by a new variable, for the type it
    /// stands for, which `hidden` gets with the opaque type's symbol. Each
    /// must turn out to have what the bounds say.
    fn hide(
        &mut self,
        ret: &Type,
        symbol: &str,
        name: &str,
        hidden: &mut Vec<(String, Type)>,
    ) -> Result<Type, Error> {
        let mut parts = Vec::new();
        for p in ret.parts() {
            parts.push(self.hide(p, symbol, name, hidden)?);
        }
        let mut parts = parts.into_iter();
        let t = ret.map(|_| parts.next().expect("as many"));
        let Type::Opaque(s, ..) = &t else {
            return Ok(t);
        };
        let d = self.traits.opaques[s].clone();
        if d.owner != symbol {
            return Ok(t);
        }
        if let Some((_, var)) = hidden.iter().find(|(h, _)| h == s) {
            return Ok(var.clone());
        }
        let var = self.fresh(
            d.span,
            format!("the type `{name}` returns"),
            "nothing in its body says".to_string(),
        );
        let swap = |u: &Type| replace_type(u, &t, &var);
        for p in &d.preds {
            let pred = Pred {
                ty: swap(&p.ty),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(pred, d.span, traits::Why::Returns(name.to_string()))?;
        }
        for (lhs, rhs) in &d.eqs {
            self.require_eq(
                swap(lhs),
                rhs.clone(),
                d.span,
                &format!("{name}'s return type"),
            )?;
        }
        hidden.push((s.clone(), var.clone()));
        Ok(var)
    }

    /// What every struct and enum is made of, by symbol.
    pub(crate) fn adts(&self) -> &Adts {
        &self.adts
    }

    /// What the borrow checker needs to know about the bodies checked.
    pub(crate) fn typed(&self) -> &Typed {
        &self.typed
    }

    /// `t` with the type variables in it replaced by what they turned out
    /// to be.
    pub(crate) fn solution(&self, t: &Type) -> Type {
        self.closure_regions(&self.solved(t))
    }

    /// `t`, with each closure type in it with as many regions as the
    /// closure has lifetime parameters, erased. The checker, which ignores
    /// regions, makes them with none, before it knows how many.
    fn closure_regions(&self, t: &Type) -> Type {
        let t = t.map(|p| self.closure_regions(p));
        match t {
            Type::Adt(symbol, regions, args)
                if let Some(c) = self.traits.closures.get(&symbol)
                    && regions.len() != c.lifetimes.len() =>
            {
                Type::Adt(symbol, vec![Region::Erased; c.lifetimes.len()], args)
            }
            t => t,
        }
    }

    /// Every trait, impl and closure.
    pub(crate) fn traits(&self) -> &Traits {
        &self.traits
    }

    /// The type of function `symbol`, which may be a method of a trait.
    pub(crate) fn scheme(&self, symbol: &str) -> &Scheme {
        if let Some(f) = self.fns.iter().find(|f| f.symbol == symbol) {
            return &f.scheme;
        }
        let methods = self.traits.traits.values().flat_map(|t| &t.methods);
        let m = methods.into_iter().find(|m| m.symbol == symbol);
        &m.expect("a function the checker saw").scheme
    }

    /// If function `id` is a method of an impl of a trait, where the
    /// trait declares it, if anywhere: renaming either renames the other.
    fn method_decl(&self, id: FnId) -> Option<Option<Span>> {
        let symbol = &self.fns[id.0].symbol;
        let d = self
            .traits
            .impls
            .iter()
            .find(|d| d.methods.iter().any(|(_, s)| s == symbol))?;
        let name = &self.fns[id.0].name.name;
        Some(self.traits.method(d.trait_name.as_ref()?, name)?.span)
    }

    /// Type `t`, written inside the function being checked, with lifetimes
    /// meaning what they mean there.
    pub(crate) fn written_type(&self, t: &TypeExpr) -> Option<Type> {
        self.resolve(t).ok()
    }

    /// Records local `l` for the borrow checker.
    fn bind(&mut self, l: &Local) {
        let bind = Bind {
            name: l.name.clone(),
            ty: l.ty.clone(),
            mutable: l.mutable,
            mut_span: l.mut_span,
            by_ref: l.by_ref,
        };
        self.typed.binds.insert(l.id(), bind);
        for site in &l.sites {
            self.typed.sites.insert(site.span, l.id());
        }
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
            if matches!(name.name.as_str(), "int" | "bool" | "Box" | "Self") {
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
            let arity = (d.generics().lifetimes.len(), d.type_params().len());
            self.arities.insert(def.symbol.clone(), arity);
        }

        let mut adts = Adts::new();
        adts.proj = std::mem::take(&mut self.adts.proj);
        // Every field's type, with the symbol of the type it's in and where
        // it was written, for the recursion checks.
        let mut field_types = Vec::new();
        let symbols: Vec<String> = self.types.iter().map(|t| t.symbol.clone()).collect();
        for ((d, _), symbol) in decls.iter().zip(&symbols) {
            let params = generics::declare(d.type_params())?;
            let lifetimes = generics::declare_lifetimes(&d.generics().lifetimes)?;
            let bounds = generics::bounds(d.generics(), &lifetimes, &params)?;
            let (preds, eqs) = self.preds(d.generics(), &params, &lifetimes, false)?;
            self.env = preds.clone();
            self.adts.proj.local = eqs.clone();
            let mut lts = Lifetimes::Fields {
                declared: &lifetimes,
            };
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
                        let t = self.resolve_with(&f.ty, &params, &mut lts)?;
                        field_types.push((symbol.as_str(), f.ty.span, t.clone()));
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
                            let t = self.resolve_with(f, &params, &mut lts)?;
                            field_types.push((symbol.as_str(), f.span, t.clone()));
                            tys.push(t);
                        }
                        variants.push((v.name.name.clone(), tys));
                    }
                    AdtKind::Enum(variants)
                }
            };
            let adt = AdtDef {
                params,
                lifetimes,
                bounds,
                preds,
                eqs,
                kind,
            };
            adts.insert(symbol.clone(), adt);
        }

        // `struct S<T> { f: fn(S<Option<T>>) -> int }` would need `S<int>`,
        // so `S<Option<int>>`, so `S<Option<Option<int>>>`...
        let mut uses = Vec::new();
        for (owner, span, t) in &field_types {
            each_adt(t, &mut |name, args| {
                uses.push(Use {
                    from: owner,
                    to: name,
                    args: args.to_vec(),
                    inside: Vec::new(),
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
        // plus... itself. It can hold itself through a pointer, which has a
        // size of its own: `Box<A>` or `&A`.
        for (owner, span, t) in &field_types {
            if contains(&adts, t, owner, &mut HashSet::new()) {
                let name = source_name(owner);
                return Err(Error::ty(
                    *span,
                    format!(
                        "`{name}` contains itself, so it would have infinite size; hold it \
                         through a pointer, like `Box<{name}>`, instead"
                    ),
                ));
            }
        }
        self.env.clear();
        adts.proj = std::mem::take(&mut self.adts.proj);
        adts.proj.local.clear();
        self.adts = adts;
        Ok(())
    }

    /// What the type name written at `span` refers to, if it's a struct or
    /// enum or an outer type parameter (see `names.rs`).
    fn type_name(&self, span: Span) -> Option<TypeName> {
        if let Some(symbol) = self.opaque_at.get(&span) {
            let d = &self.traits.opaques[symbol];
            let (lifetimes, params) = (d.lifetimes.clone(), d.params.clone());
            return Some(TypeName::Opaque(symbol.clone(), lifetimes, params));
        }
        match self.type_names.get(&span)? {
            Scoped::Adt(symbol) => {
                let (lts, arity) = *self.arities.get(symbol)?;
                Some(TypeName::Adt(symbol.clone(), lts, arity))
            }
            Scoped::OuterParam(keyword, item) => Some(TypeName::OuterParam(keyword, item.clone())),
        }
    }

    /// The symbol of the struct or enum that the type name written at
    /// `span` refers to, if it's one.
    fn adt_symbol(&self, span: Span) -> Option<&str> {
        match self.type_names.get(&span)? {
            Scoped::Adt(symbol) => Some(symbol),
            Scoped::OuterParam(..) => None,
        }
    }

    /// A type written where type parameters `params` are in scope, and
    /// lifetimes mean what `lts` says.
    fn resolve_with(
        &self,
        t: &TypeExpr,
        params: &[String],
        lts: &mut Lifetimes,
    ) -> Result<Type, Error> {
        resolve(
            t,
            &|_, span| self.type_name(span),
            &|s, tr, name, span| self.projection(s, tr, name, span),
            params,
            lts,
        )
    }

    /// A type written inside the function being checked.
    fn resolve(&self, t: &TypeExpr) -> Result<Type, Error> {
        let mut lts = Lifetimes::Body {
            declared: &self.lifetimes,
        };
        self.resolve_with(t, &self.type_params, &mut lts)
    }

    /// Rejects polymorphic recursion between functions: see
    /// `generics::polymorphic_recursion`. A call of a trait's method may
    /// run any impl's that could be for its `Self`, so it uses each of
    /// those (`traits::dispatch`).
    ///
    /// Invariant: every copy monomorphisation can make is of a use here.
    /// So every call or naming of a generic function a body can make, at
    /// its type arguments, is in `info.instances`: calls, method calls,
    /// paths, and `==`, which calls `<Eq>::eq`. And so is every call made
    /// by something that isn't a function of the program: a derived `Eq`'s
    /// `==` on each field, below. Whatever makes monomorphisation copy
    /// something new must add its uses here too.
    ///
    /// A projection in a type argument, `f::<T::Out>`, or as the `Self` of
    /// a method call, can stand for a type as much bigger than `T` as an
    /// impl likes, so an edge through it has unbounded weight (`dispatch`,
    /// `generics::polymorphic_recursion`), and any cycle through it is
    /// rejected. The copies of types that impls' associated types make are
    /// seen to by `Checker::assoc_regular`.
    ///
    /// A closure's type is a type constructor applied to the type parameters
    /// in scope, so passing a closure that holds a `G` where a `G` was, as
    /// `f(|| g())` in `f<G>`, is an edge that grows, like `f::<Box<G>>`.
    /// So is an opaque type, applied to its function's type parameters:
    /// `g(wrap(x))` in `g<T>`, for `wrap<T>(x: T) -> impl Discard`. What it
    /// stands for is a finite type of those, which holds no opaque type
    /// that leads back to it (`opaque_cycles`), so replacing each with
    /// that, as monomorphisation does, makes no more copies than there are
    /// of the opaque types.
    /// What a closure's body uses is in the function it's in, by where it
    /// is, and is copied with it.
    fn no_polymorphic_recursion(&self) -> Result<(), Error> {
        // The `eq` of each derived `Eq`: not a function of the program, but
        // it does what one would, `==` on each field, so it's a node of its
        // own (see `dispatch`).
        let derived_eq: Vec<Option<String>> = self
            .traits
            .impls
            .iter()
            .map(|d| {
                let eq = d.derived && d.trait_name.as_deref() == Some("Eq");
                eq.then(|| format!("<{} as Eq>::eq", d.self_ty.symbols()))
            })
            .collect();
        let eq = traits::trait_method_symbol("Eq", "eq");
        let mut uses = Vec::new();
        for (k, d) in self.traits.impls.iter().enumerate() {
            let (Some(from), Type::Adt(symbol, ..)) = (&derived_eq[k], &d.self_ty) else {
                continue;
            };
            for (_, f) in traits::fields(&self.adts[symbol].kind) {
                let args = [f];
                uses.extend(self.dispatch(from, &d.params, &eq, &args, &derived_eq, d.span));
            }
        }
        // Which function each use is in, from where they are.
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
                let mut dispatched = self.dispatch(
                    &from.symbol,
                    &from.scheme.params,
                    to,
                    args,
                    &derived_eq,
                    *span,
                );
                if dispatched.is_empty() {
                    dispatched.push(Use {
                        from: &from.symbol,
                        to,
                        args: args.clone(),
                        inside: Vec::new(),
                        span: *span,
                    });
                }
                uses.extend(dispatched);
            }
        }
        // In source order, so the first one is reported.
        uses.sort_by_key(|u| u.span.start);
        let mut params: HashMap<&str, &[String]> = self
            .fns
            .iter()
            .map(|f| (f.symbol.as_str(), f.scheme.params.as_slice()))
            .collect();
        for (d, eq) in self.traits.impls.iter().zip(&derived_eq) {
            if let Some(eq) = eq {
                params.insert(eq, &d.params);
            }
        }
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
    ) -> Result<Option<(Type, Res)>, Error> {
        // Notes are for the name, not any `::<..>` after it.
        let at = Span::new(span.start, span.start + name.len());
        let (scheme, def, symbol, builtin) = match self.binding(name) {
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
                let (ty, def) = (local.ty.clone(), local.id());
                self.note(
                    at,
                    NoteKind::Var(name.to_string()),
                    Some(ty.clone()),
                    Some(def),
                );
                return Ok(Some((ty, Res::Local(def))));
            }
            Some(Binding::Fn(id)) => {
                self.uses().fns.insert(id);
                let f = &self.fns[id.0];
                let scheme = f.scheme.clone();
                (scheme, Some(f.name.span), Some(f.symbol.clone()), None)
            }
            Some(Binding::Builtin(b)) => (builtin_scheme(b), None, None, Some(b)),
        };
        let args = self.type_args(name, &scheme.params, targs, span, "(..)")?;
        let s = Subst::new(&scheme.params, &args);
        let ty = s.apply(&scheme.ty);
        // What it needs of its type arguments, by its bounds.
        for p in &scheme.preds {
            let pred = Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(
                pred,
                at,
                traits::Why::Bound(name.to_string(), p.ty.to_string()),
            )?;
        }
        for (lhs, rhs) in &scheme.eqs {
            self.require_eq(s.apply(lhs), s.apply(rhs), at, name)?;
        }
        let res = match (&symbol, builtin) {
            (Some(symbol), _) => Res::Fn(symbol.clone(), args.clone()),
            (None, Some(b)) => Res::Builtin(b, args.clone()),
            (None, None) => unreachable!("a function or a built-in"),
        };
        let instance = symbol.map(|symbol| {
            self.info.symbols.insert(span, symbol.clone());
            (span, Instance::Fn(symbol, args))
        });
        let kind = NoteKind::Fn(name.to_string());
        self.note_use(at, kind, ty.clone(), def, instance, targs.is_empty());
        Ok(Some((ty, res)))
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
        let mut lts = Lifetimes::Body {
            declared: &self.lifetimes,
        };
        let ty = self.resolve_with(&t, &params, &mut lts)?;
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
    /// that were never used. A name starting with `_` says that's on purpose,
    /// and a method that doesn't use `self` can't do without it.
    fn end_scope(&mut self, scope: usize) {
        for local in self.locals.split_off(scope) {
            if !local.used && !local.name.starts_with('_') && local.name != "self" {
                self.unused.push((self.warnings.len(), local.ty.clone()));
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
    /// Blocks, `match` and `loop` pass `want` on to their results, so that
    /// reaches closures inside them too.
    ///
    /// A `!` fits whatever is wanted (T-Never).
    fn check(&mut self, e: &Expr, want: &Type) -> Result<(), Error> {
        self.here = e.span;
        let got = match &e.kind {
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), Some(want))?,
            ExprKind::Match(scrutinee, arms) => self.match_expr(e, scrutinee, arms, Some(want))?,
            ExprKind::Loop(label, body) => self.loop_expr(label, None, body, Some(want))?,
            ExprKind::Lambda(l) => {
                let sig = match self.shallow(want) {
                    // A function type is wanted, which a closure can only be
                    // if it captures nothing: what it captures has to be
                    // somewhere.
                    Type::Fn(sig) => match self.captured(l).first() {
                        Some(id) => {
                            let name = &self.typed.binds[id].name;
                            let at = first_use(&l.body, name).unwrap_or(l.body.span);
                            let ps: Vec<String> =
                                sig.params.iter().map(|p| p.to_string()).collect();
                            return Err(Error::ty(
                                e.span,
                                format!(
                                    "expected `{}`, found a closure that captures `{name}`",
                                    Type::Fn(sig.clone())
                                ),
                            )
                            .with_note(at, format!("`{name}` is captured here"))
                            .with_note(
                                e.span,
                                format!(
                                    "a function type only holds functions, and closures that \
                                     capture nothing; to take this, take a type parameter \
                                     `F: Fn({}) -> {}`",
                                    ps.join(", "),
                                    sig.ret
                                ),
                            ));
                        }
                        None if sig.params.len() == l.params.len() => Some((sig.params, *sig.ret)),
                        None => None,
                    },
                    // What a bound on a type to be inferred says calls of it
                    // take and give: `F: Fn(int) -> int` for `apply`'s `F`
                    // in `apply(|x| x + 1)`.
                    Type::Var(v) => self.wanted_sig(v, l.params.len()),
                    _ => None,
                };
                self.lambda(l, sig)?
            }
            // `&x` where a `&T` is wanted: `x` should be a `T`.
            ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) => match self.shallow(want) {
                Type::Ref(_, m, inner) if m == (*op == UnOp::RefMut) => {
                    self.check(x, &inner)?;
                    Type::Ref(Region::Erased, m, inner)
                }
                _ => self.synth(e)?,
            },
            _ => self.synth(e)?,
        };
        self.typed.exprs.insert(e, got.clone());
        self.note_never(e, &got);
        if let Some(target) = self.coerce(e, &got, want) {
            return match self.unify(&target, want) {
                Ok(()) => Ok(()),
                Err(_) => Err(Error::ty(
                    result_span(e),
                    format!(
                        "expected `{}`, found `{}`",
                        self.solved(want),
                        self.solved(&got)
                    ),
                )),
            };
        }
        self.fits(&got, want, result_span(e), |got, want| {
            format!("expected `{want}`, found `{got}`")
        })
    }

    //   Γ ⊢ e : !
    //  ----------- (T-Never)
    //   Γ ⊢ e : T
    //
    // An expression of type `!` never finishes, so it never gives a value of
    // the wrong type, or any: it can be used as one of any type. That's
    // *ex falso quodlibet*, from nothing, anything.
    //
    // This rule isn't syntax-directed: it applies to any expression. So it's
    // only used where the type wanted is known, as `fits`, rather than by
    // `unify`, which would have to guess.

    /// Like `unify_or`, but `got` fits if it's `!`, by T-Never, whatever
    /// `want` is. If `want` is a type not known yet, it's noted, to be `!`
    /// if nothing else decides it (see `solve_all`): in `let x = id(return);`
    /// the type argument of `id` is `!`.
    fn fits(
        &mut self,
        got: &Type,
        want: &Type,
        span: Span,
        mismatch: impl FnOnce(&Type, &Type) -> String,
    ) -> Result<(), Error> {
        if self.shallow(got) != Type::Never {
            return self.unify_or(got, want, span, mismatch);
        }
        if let Type::Var(_) = self.shallow(want) {
            self.diverging.push(want.clone());
        }
        Ok(())
    }

    /// The type of two branches of types `a` and `b`, one of which runs:
    /// the other's, if one is `!`, or else both, which must be the same.
    fn join(
        &mut self,
        a: &Type,
        b: &Type,
        span: Span,
        mismatch: impl FnOnce(&Type, &Type) -> String,
    ) -> Result<Type, Error> {
        match (self.shallow(a), self.shallow(b)) {
            (Type::Never, _) => Ok(b.clone()),
            (_, Type::Never) => Ok(a.clone()),
            _ => {
                self.unify_or(a, b, span, mismatch)?;
                Ok(a.clone())
            }
        }
    }

    /// Notes that the code checked so far diverges if `e` has type `t`, a
    /// `!`, which has no values to finish with.
    fn note_never(&mut self, e: &Expr, t: &Type) {
        if self.shallow(t) == Type::Never && self.diverges.is_none() {
            self.diverges = Some(e.span);
        }
    }

    /// If `e`, a reference of type `got`, is wanted as a reference of type
    /// `want` that it isn't, the reference it can be *coerced* to: it's
    /// borrowed again, as `&**e` say. A `&mut T` can be used as a `&T`,
    /// since there's nothing a `&T` can do that a `&mut T` can't, and a
    /// `&Box<T>` or `&&T` as a `&T`, by following the box or reference to
    /// what it points to, as Rust's *deref coercion* does. Recorded for the
    /// borrow checker, and for `lift` to write out.
    fn coerce(&mut self, e: &Expr, got: &Type, want: &Type) -> Option<Type> {
        let (Type::Ref(_, gm, a), Type::Ref(_, wm, b)) = (self.shallow(got), self.shallow(want))
        else {
            return None;
        };
        if wm && !gm {
            return None;
        }
        let mut a = *a;
        let mut derefs = 0;
        loop {
            let (sa, sb) = (self.shallow(&a), self.shallow(&b));
            if matches!(sa, Type::Var(_)) || matches!(sb, Type::Var(_)) || sa.same_shape(&sb) {
                break;
            }
            match sa {
                Type::Box(inner) => a = *inner,
                Type::Ref(_, m, inner) if m || !wm => a = *inner,
                _ => break,
            }
            derefs += 1;
        }
        if derefs == 0 && gm == wm {
            return None;
        }
        self.typed.coerced.insert(e, (derefs, wm));
        self.info.coercions.insert(e.span, (derefs, wm));
        Some(Type::Ref(Region::Erased, wm, Box::new(a)))
    }

    /// Computes ("synthesizes") the type of `e`.
    fn synth(&mut self, e: &Expr) -> Result<Type, Error> {
        self.here = e.span;
        let t = self.synth_inner(e)?;
        self.settle_goals()?;
        self.typed.exprs.insert(e, t.clone());
        self.note_never(e, &t);
        Ok(t)
    }

    fn synth_inner(&mut self, e: &Expr) -> Result<Type, Error> {
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
            ExprKind::Var(name, targs) => {
                let Some((t, res)) = self.lookup(name, targs, e.span)? else {
                    return Err(Error::ty(e.span, format!("unknown variable `{name}`")));
                };
                self.typed.vars.insert(e, res);
                Ok(t)
            }

            //   Γ ⊢ e : int            Γ ⊢ e : bool
            //  -------------- (T-Neg)  -------------- (T-Not)
            //   Γ ⊢ -e : int            Γ ⊢ !e : bool
            ExprKind::Unary(op @ (UnOp::Neg | UnOp::Not), x) => {
                let t = match op {
                    UnOp::Neg => Type::Int,
                    _ => Type::Bool,
                };
                self.check(x, &t)?;
                Ok(t)
            }

            //        Γ ⊢ e : T                   Γ ⊢ e : T
            //  ---------------------- (T-Ref)  -------------------------- (T-RefMut)
            //   Γ ⊢ &e : &'r T                Γ ⊢ &mut e : &'r mut T
            //
            // The region `'r` is how long the borrow lasts. Nothing here
            // decides that; the borrow checker works it out, and checks that
            // `e` is a place that lives that long and isn't used in a way
            // that clashes with the borrow while it lasts. `&` of a value
            // that isn't a place borrows a temporary holding it.
            ExprKind::Unary(op @ (UnOp::Ref | UnOp::RefMut), x) => {
                let t = self.synth(x)?;
                Ok(Type::Ref(Region::Erased, *op == UnOp::RefMut, Box::new(t)))
            }

            //   Γ ⊢ e : &'r T         Γ ⊢ e : &'r mut T         Γ ⊢ e : Box<T>
            //  --------------- (T-Deref) ------------------- ------------------
            //    Γ ⊢ *e : T              Γ ⊢ *e : T              Γ ⊢ *e : T
            ExprKind::Unary(UnOp::Deref, x) => {
                let t = self.synth(x)?;
                match self.shallow(&t) {
                    Type::Ref(_, _, inner) | Type::Box(inner) => Ok(*inner),
                    Type::Var(_) => Err(Error::ty(
                        x.span,
                        "the type of this must be known to use `*` on it; add a type annotation",
                    )),
                    t => Err(Error::ty(
                        e.span,
                        format!("can't use `*` on `{t}`: only on a reference or a box"),
                    )),
                }
            }

            //   Γ ⊢ p : T    p is a place    Γ ⊢ e : T
            //  ---------------------------------------- (T-Assign)
            //               Γ ⊢ p = e : ()
            //
            // Whether `p` may be changed, by being declared `mut` or being
            // behind a `&mut`, is for the borrow checker.
            ExprKind::Assign(op, place, value) => {
                let not_place = || {
                    Error::ty(
                        place.span,
                        "can't assign to this: only to a variable, a field, or what `*` points to",
                    )
                };
                if !place.is_place() {
                    return Err(not_place());
                }
                let t = self.synth(place)?;
                if let ExprKind::Var(..) = place.kind
                    && !matches!(
                        self.typed.vars.get(&std::ptr::from_ref(&**place)),
                        Some(Res::Local(_))
                    )
                {
                    return Err(not_place());
                }
                match op {
                    None => {
                        self.check(value, &t)?;
                    }
                    Some(op) => {
                        self.unify_or(&t, &Type::Int, place.span, |t, _| {
                            format!("`{op}=` works on an `int`, not on `{t}`")
                        })?;
                        self.check(value, &Type::Int)?;
                    }
                }
                Ok(Type::Unit)
            }

            //   Γ ⊢ c : bool    Γ ⊢ body : ()
            //  ------------------------------- (T-While)
            //     Γ ⊢ while c { body } : ()
            //
            // The body may not run at all, so whether it diverges doesn't
            // matter to what comes after, and nor does a `break` in it.
            ExprKind::While(label, c, body) => self.loop_expr(label, Some(c), body, None),

            ExprKind::Loop(label, body) => self.loop_expr(label, None, body, None),

            //   Γ ⊢ e : R    R is what the function returns
            //  -------------------------------------------- (T-Return)
            //               Γ ⊢ return e : !
            //
            // `return` is short for `return ()`. Its own type is `!`: it
            // never finishes, as control leaves for the caller.
            ExprKind::Return(value) => {
                let ret = match self.ret.clone() {
                    Some(ret) => ret,
                    None => {
                        let what = "the type of what this closure returns".to_string();
                        let ret = self.fresh(
                            e.span,
                            what,
                            "annotate it, e.g. `|x| -> int { .. }`".into(),
                        );
                        self.ret = Some(ret.clone());
                        ret
                    }
                };
                match value {
                    Some(v) => {
                        self.check(v, &ret)?;
                    }
                    None => self.unify_or(&Type::Unit, &ret, e.span, |_, ret| {
                        format!("a `return` with no value returns `()`, but this returns `{ret}`")
                    })?,
                }
                Ok(Type::Never)
            }

            //   Γ ⊢ e : T    the loop around is a `loop` of type T
            //  --------------------------------------------------- (T-Break)
            //                 Γ ⊢ break e : !
            //
            //   the loop around is a `loop` of type () or a `while`
            //  ---------------------------------------------------- (T-Break-Unit)
            //                  Γ ⊢ break : !
            //
            //   there's a loop around
            //  ----------------------- (T-Continue)
            //   Γ ⊢ continue : !
            //
            // The loop is the innermost, or with a label, the one with it.
            ExprKind::Break(label, value) => {
                let i = self.target(e.span, "break", label.as_ref())?;
                let Target::Loop(lp) = &self.loops[i] else {
                    unreachable!("a loop")
                };
                if lp.is_while && value.is_some() {
                    return Err(Error::ty(
                        e.span,
                        "a `break` with a value only works in a `loop`: a `while` loop can end \
                         without a `break`, and then it has no value to give",
                    ));
                }
                let target = lp.value.clone().or_else(|| lp.want.clone());
                let t = match (value, target) {
                    (Some(v), Some(t)) => {
                        self.check(v, &t)?;
                        Some(t)
                    }
                    (Some(v), None) => Some(self.synth(v)?),
                    (None, target) => {
                        if let Some(t) = &target {
                            self.unify_or(&Type::Unit, t, e.span, |_, t| {
                                format!(
                                    "a `break` with no value gives the loop `()`, but it should \
                                     be `{t}`"
                                )
                            })?;
                        }
                        Some(Type::Unit)
                    }
                };
                // A value of type `!` never gets as far as the `break`.
                let t = t.filter(|t| self.shallow(t) != Type::Never);
                if let (Some(t), Target::Loop(lp)) = (t, &mut self.loops[i])
                    && lp.value.is_none()
                {
                    lp.value = Some(t);
                }
                Ok(Type::Never)
            }
            ExprKind::Continue(label) => {
                self.target(e.span, "continue", label.as_ref())?;
                Ok(Type::Never)
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
                //
                // The right side may not run, so whether it diverges doesn't
                // matter to what comes after.
                BinOp::And | BinOp::Or => {
                    self.check(l, &Type::Bool)?;
                    let before = self.diverges.take();
                    self.check(r, &Type::Bool)?;
                    self.diverges = before;
                    Ok(Type::Bool)
                }

                //   Γ ⊢ l : T   Γ ⊢ r : T   T: Eq
                //  ------------------------------- (T-Eq) for == !=
                //         Γ ⊢ l == r : bool
                //
                // `==` is `Eq::eq(&l, &r)`, and `!=` its negation, so T
                // must implement `Eq`. `int`, `bool` and `()` do, and
                // references and boxes if what they point to does: they
                // compare that, not addresses. A struct or enum does if
                // it says so, with `#[derive(Eq)]` to compare field by
                // field, or an impl of its own. Functions don't, and can't:
                // whether two functions do the same for every argument is
                // undecidable, and comparing them by address would make
                // `==` depend on how the compiler laid out the code. A type
                // parameter `T` does if a bound says so, `T: Eq`.
                //
                // If T isn't fully known yet, as in `|a, b| a == b`, the
                // obligation waits until it is, at the end of the function.
                //
                // Being a call of `eq`, it's a use of it, at `T`: for
                // monomorphisation to make the copies of the impls' `eq`s it
                // may call, and for the check against polymorphic recursion
                // to see it.
                BinOp::Eq | BinOp::Ne => {
                    let t = self.synth(l)?;
                    self.check(r, &t)?;
                    let eq = traits::trait_method_symbol("Eq", "eq");
                    self.pending
                        .push((e.span, Instance::Fn(eq, vec![t.clone()])));
                    let pred = Pred {
                        ty: t,
                        trait_name: "Eq".into(),
                    };
                    self.oblige(pred, e.span, traits::Why::Compare(*op))?;
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
                    // A closure, or what a bound says can be called, or a
                    // reference to one: what its signature says. How the
                    // call gets at it is worked out once that's known
                    // (`settle_calls`).
                    t => {
                        let sig = Type::Proj(
                            Box::new(t.clone()),
                            "FnOnce".into(),
                            crate::traits::SIG.into(),
                        );
                        let Type::Fn(sig) = self.shallow(&sig) else {
                            let hint = match &t {
                                Type::Param(p) => format!(
                                    "; to call it, say what it takes and gives with a bound, \
                                     like `{p}: Fn(int) -> int`"
                                ),
                                _ => String::new(),
                            };
                            return Err(Error::ty(
                                f.span,
                                format!("`{t}` is not a function{hint}"),
                            ));
                        };
                        let pred = Checker::callable_pred(&t);
                        self.oblige(pred, f.span, traits::Why::Call)?;
                        self.calls.push((e, t, e.span));
                        (sig.params, *sig.ret)
                    }
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

            //   Γ ⊢ e : &..&S    m is a method of S of type fn(R, T1, .., Tn) -> U
            //   R is S, &S or &mut S    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
            //  ------------------------------------------------------------------ (T-Method)
            //                   Γ ⊢ e.m(a1, .., an) : U
            //
            // A method call is a call of the method with `e` as its first
            // argument, its *receiver*: what `e` points to, through any
            // number of references and boxes, as a field is (T-Field), and
            // then borrowed, if the method takes `&self` or `&mut self`. So
            // `v.push(1)` is `<V>::push(&mut v, 1)`, and for `r: &&V`,
            // `r.len()` is `<V>::len(&**r)`. That's all the adjusting there
            // is: at most one `&` or `&mut`.
            //
            // Which method `m` is depends on S, so S must be known here, as
            // for a field. See `lookup_method` for how it's found.
            ExprKind::MethodCall(x, name, targs, args) => {
                let t = self.synth(x)?;
                let mut base = self.solved(&t);
                let mut derefs = 0;
                while let Type::Ref(_, _, inner) | Type::Box(inner) = base {
                    base = *inner;
                    derefs += 1;
                }
                if let Type::Var(_) = base {
                    return Err(Error::ty(
                        x.span,
                        format!(
                            "the type of this must be known to call its method `{}`; add a \
                             type annotation",
                            name.name
                        ),
                    ));
                }
                let found = match self.method_of(&base, name, targs) {
                    Ok(found) => found,
                    // `s.f(1)` for a field `f` that holds a function.
                    Err(err) => match struct_fields(&self.adts, &base)
                        .and_then(|fs| fs.into_iter().find(|(f, _)| *f == name.name))
                    {
                        Some((_, Type::Fn(_))) => {
                            let field = Span::new(x.span.start, name.span.end);
                            return Err(Error::ty(
                                name.span,
                                format!(
                                    "`{base}` has no method `{0}`, but it has a field `{0}`, which \
                                     holds a function; to call that, put the field in \
                                     parentheses",
                                    name.name
                                ),
                            )
                            .with_fix(Fix::new(
                                "put the field in parentheses",
                                vec![Edit::insert(field.start, "("), Edit::insert(field.end, ")")],
                            )));
                        }
                        _ => return Err(err),
                    },
                };
                if !found.receiver {
                    return Err(Error::ty(
                        name.span,
                        format!(
                            "`{}` doesn't take `self`, so it can't be called with `.`; call it \
                             as `{}::{}(..)`",
                            name.name,
                            source_name(&base.to_string()),
                            name.name
                        ),
                    ));
                }
                let Type::Fn(sig) = &found.ty else {
                    unreachable!("a method has a function type")
                };
                let autoref = match self.shallow(&sig.params[0]) {
                    Type::Ref(_, m, _) => Some(m),
                    _ => None,
                };
                let receiver = match autoref {
                    Some(m) => Type::Ref(Region::Erased, m, Box::new(base.clone())),
                    None => base.clone(),
                };
                self.unify(&receiver, &sig.params[0])
                    .unwrap_or_else(|_| unreachable!("the method is of what's before the `.`"));
                let params = &sig.params[1..];
                if params.len() != args.len() {
                    return Err(Error::ty(
                        e.span,
                        format!(
                            "method `{}` takes {} but {} {} supplied",
                            name.name,
                            plural(params.len(), "argument"),
                            args.len(),
                            if args.len() == 1 { "was" } else { "were" },
                        ),
                    ));
                }
                let ret = (*sig.ret).clone();
                let params = params.to_vec();
                self.resolved(name, &found, targs.is_empty());
                for (arg, param) in args.iter().zip(&params) {
                    self.check(arg, param)?;
                }
                let res = Res::Fn(found.symbol.clone(), found.args.clone());
                self.typed.methods.insert(e, (res, derefs, autoref));
                let resolved = Resolved {
                    symbol: found.symbol.clone(),
                    derefs,
                    autoref,
                };
                self.info.methods.insert(name.span, resolved);
                if let Some(pred) = found.of_trait {
                    self.oblige(pred, name.span, traits::Why::Method(name.name.clone()))?;
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
            //
            // If one branch is a `!`, the `if` has the other's type (T-Never).
            // It diverges if both branches do.
            ExprKind::If(c, then, els) => {
                self.check(c, &Type::Bool)?;
                let before = self.diverges.take();
                let t = match els {
                    Some(els) => {
                        let t = self.synth(then)?;
                        let then_diverges = self.diverges.take();
                        let u = self.synth(els)?;
                        let both = then_diverges.is_some() && self.diverges.is_some();
                        self.diverges = before.or(both.then_some(e.span));
                        self.join(&t, &u, result_span(els), |t, u| {
                            format!("`if` and `else` have different types: `{t}` and `{u}`")
                        })?
                    }
                    None => {
                        let t = self.synth(then)?;
                        self.diverges = before;
                        self.fits(&t, &Type::Unit, result_span(then), |t, _| {
                            format!("`if` without `else` must have type `()`, found `{t}`")
                        })?;
                        Type::Unit
                    }
                };
                Ok(t)
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
                let Type::Adt(symbol, ..) = &ty else {
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
            //
            // If e is a reference or a box, the field is of what it points
            // to, at any depth: `r.x` for `r: &Point` is `(*r).x`.
            ExprKind::Field(x, field) => {
                let t = self.synth(x)?;
                let mut t = self.solved(&t);
                let mut derefs = 0;
                while let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                    t = *inner;
                    derefs += 1;
                }
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
                match fields
                    .into_iter()
                    .enumerate()
                    .find(|(_, (f, _))| *f == field.name)
                {
                    Some((i, (_, ft))) => {
                        if let Type::Adt(s, ..) = &t {
                            self.note_field(&s.clone(), field, &ft);
                        }
                        self.typed.fields.insert(e, (derefs, i));
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
            ExprKind::Variant(ty, targs, variant, args, ..) if self.is_box(ty) => {
                self.box_new(e, targs, variant, args)
            }
            ExprKind::Variant(ty, targs, method, args, parens) if !self.is_variant(ty, method) => {
                self.path(e, ty, targs, method, args, *parens)
            }
            ExprKind::Variant(ty, targs, variant, args, ..) => {
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
                }
                let Type::Adt(symbol, ..) = &t else {
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

    //   Γ ⊢ body : ()    the `break`s in body give T
    //  ---------------------------------------------- (T-Loop)
    //            Γ ⊢ loop { body } : T
    //
    //   Γ ⊢ body : ()    no `break` in body gets out
    //  -------------------------------------------- (T-Loop-Never)
    //            Γ ⊢ loop { body } : !
    //
    // The only way out of a `loop` is a `break`, so its value is what they
    // give it. With none, it never ends.
    //
    // `loop_expr` checks a `while` too, with its condition `cond`: T-While.
    // A `break` in the condition is for a loop around the `while`, as the
    // condition isn't part of its body. A loop's label can't be one a loop
    // around it has, which `break` couldn't get to past it, and one that
    // nothing uses gets a warning.
    fn loop_expr(
        &mut self,
        label: &Option<Label>,
        cond: Option<&Expr>,
        body: &Expr,
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        if let Some(c) = cond {
            self.check(c, &Type::Bool)?;
        }
        if let Some(l) = label {
            let outer = self.loops.iter().rev().map_while(|t| match t {
                Target::Loop(lp) => Some(lp),
                Target::Closure => None,
            });
            if let Some(outer) = outer
                .filter_map(|lp| lp.label.as_ref())
                .find(|o| o.name.name == l.name.name)
            {
                let name = &l.name.name;
                return Err(Error::ty(
                    l.name.span,
                    format!(
                        "a loop around this one is labelled `{name}` already, so a `break {name}` \
                         in here couldn't get to it; give this one another label"
                    ),
                )
                .with_note(outer.name.span, format!("`{name}` is that loop's label")));
            }
        }
        let before = self.diverges.take();
        self.loops.push(Target::Loop(Box::new(LoopCx {
            label: label.clone(),
            used: false,
            is_while: cond.is_some(),
            want: want.cloned(),
            value: None,
        })));
        let what = match cond {
            Some(_) => "a `while` loop's body",
            None => "a `loop`'s body",
        };
        let result = self.loop_body(body, what);
        let Some(Target::Loop(lp)) = self.loops.pop() else {
            unreachable!("pushed a loop")
        };
        self.diverges = before;
        result?;
        if let (Some(l), false) = (&lp.label, lp.used) {
            self.warnings.push(unused_label(l));
        }
        Ok(match cond {
            Some(_) => Type::Unit,
            None => lp.value.unwrap_or(Type::Never),
        })
    }

    /// Checks the body of a `while` or `loop`, which must be a `()`, as a
    /// value would have nowhere to go. `what` names it in the message.
    fn loop_body(&mut self, body: &Expr, what: &str) -> Result<(), Error> {
        let t = self.synth(body)?;
        self.fits(&t, &Type::Unit, result_span(body), |t, _| {
            format!(
                "expected `()`, found `{t}`: {what} has nowhere for a value to go; add `;` to \
                 discard it"
            )
        })
    }

    /// The index in `loops` of the loop a `break` or `continue` (`what`) at
    /// `span` goes to: the innermost, or the one with `label`, which is then
    /// used.
    fn target(&mut self, span: Span, what: &str, label: Option<&Ident>) -> Result<usize, Error> {
        let closure = || {
            Error::ty(
                span,
                format!(
                    "can't `{what}` out of a closure: its body is a function of its own, so the \
                     loop around it isn't its to leave"
                ),
            )
        };
        let Some(label) = label else {
            return match self.loops.last() {
                Some(Target::Loop(_)) => Ok(self.loops.len() - 1),
                Some(Target::Closure) => Err(closure()),
                None => Err(Error::ty(span, format!("`{what}` outside of a loop"))),
            };
        };
        let named = |t: &Target| match t {
            Target::Loop(lp) => lp.label.as_ref().is_some_and(|l| l.name.name == label.name),
            Target::Closure => false,
        };
        let Some(i) = self.loops.iter().rposition(named) else {
            return Err(Error::ty(
                label.span,
                format!("no loop around this `{what}` is labelled `{}`", label.name),
            ));
        };
        if self.loops[i..].iter().any(|t| matches!(t, Target::Closure)) {
            return Err(closure());
        }
        if let Target::Loop(lp) = &mut self.loops[i] {
            lp.used = true;
        }
        Ok(i)
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
    //
    // A `return` in the body returns from the closure, and nothing in it can
    // `break` out of a loop around it: the body is a function of its own.
    // Whether it diverges doesn't matter to the code around: making a
    // closure doesn't run it.
    fn lambda(&mut self, l: &Arc<Lambda>, sig: Option<(Vec<Type>, Type)>) -> Result<Type, Error> {
        let outer_ret = self.ret.take();
        let outer_diverges = self.diverges.take();
        self.loops.push(Target::Closure);
        let result = self.lambda_inner(l, sig);
        self.loops.pop();
        self.ret = outer_ret;
        self.diverges = outer_diverges;
        result
    }

    fn lambda_inner(
        &mut self,
        l: &Arc<Lambda>,
        sig: Option<(Vec<Type>, Type)>,
    ) -> Result<Type, Error> {
        // What it captures: the variables around it its body uses. How, is
        // worked out once their types are known (see `closures`).
        let captured = self.captured(l);
        self.typed.captures.insert(Arc::as_ptr(l), captured.clone());
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
            let mut local = Local::param(name, t, p.name.span);
            local.mutable = p.mutable.is_some();
            local.mut_span = p.mutable.map(|m| Span::new(m.start, p.name.span.start));
            self.note_binds(std::slice::from_ref(&local), p.ty.is_none());
            self.bind(&local);
            self.locals.push(local);
        }
        let wanted = sig.map(|(_, r)| r);
        let ret = match &l.ret {
            Some(ann) => Some(self.annotation(ann, wanted)?),
            None => wanted,
        };
        self.ret = ret.clone();
        let ret = match ret {
            Some(r) => {
                self.check(&l.body, &r)?;
                r
            }
            // Its result is that of the body, and of any `return` in it.
            None => {
                let t = self.synth(&l.body)?;
                match self.ret.clone() {
                    Some(r) => {
                        self.fits(&t, &r, result_span(&l.body), |t, r| {
                            format!("this closure returns `{r}`, but its body is `{t}`")
                        })?;
                        r
                    }
                    None => t,
                }
            }
        };
        self.end_scope(scope);
        match captured.is_empty() {
            true => Ok(Type::func(params, ret)),
            false => Ok(self.closure_type(l, captured, params, ret)),
        }
    }

    /// The variables around closure `l` its body uses, by id.
    fn captured(&self, l: &Lambda) -> Vec<Span> {
        l.captures
            .iter()
            .filter_map(|name| match self.binding(name) {
                Some(Binding::Local(i)) if i >= self.barrier => Some(self.locals[i].id()),
                _ => None,
            })
            .collect()
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
    //
    //   Γ ⊢ { stmts } diverges
    //  ------------------------ (T-Block-Never)
    //    Γ ⊢ { stmts } : !
    //
    // A block with no tail is a `()`, unless it never gets to its end, say
    // as it ends with a `return`: then it's a `!`. Code that comes after
    // something that diverges never runs, and gets a warning.
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
        // If the code before the block diverges, that's for the code
        // around it to warn about.
        let mut warned = self.diverges.is_some();
        for (k, stmt) in stmts.iter().enumerate() {
            if !warned
                && self.diverges.is_some()
                && let Some(span) = stmt_span(stmt)
            {
                self.warnings.push(unreachable_code(span));
                warned = true;
            }
            match stmt {
                Stmt::Fn(f) => {
                    let fix = to_closure(f, stmts, k, tail);
                    self.nested(f, fix)?;
                }
                // Checked with the others, before any function.
                Stmt::Type(_) => {}
                Stmt::Let { pat, ty, init, .. } => {
                    // `let x;`: given its value later, by assignment, which
                    // can only give a whole variable one.
                    let deferred = match (&pat.kind, init) {
                        (_, Some(_)) => None,
                        (PatternKind::Bind(name, _), None) => Some(name),
                        (_, None) => {
                            return Err(Error::ty(
                                pat.span,
                                "a `let` with no value can only declare a variable, like \
                                 `let x;`, as assignment gives a variable its value",
                            ));
                        }
                    };
                    let t = match (ty, init) {
                        (Some(ann), init) => {
                            let t = self.resolve(ann)?;
                            self.mention(&t);
                            if let Some(init) = init {
                                self.check(init, &t)?;
                            }
                            t
                        }
                        (None, Some(init)) => self.synth(init)?,
                        (None, None) => {
                            let name = deferred.expect("a variable");
                            self.fresh(
                                pat.span,
                                format!("the type of `{name}`"),
                                format!("annotate it, e.g. `let {name}: int;`"),
                            )
                        }
                    };
                    let mut binds = Vec::new();
                    let p = self.pattern(pat, &t, None, &mut binds)?;
                    self.note_binds(&binds, ty.is_none());
                    binds.iter().for_each(|l| self.bind(l));
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
                        ExprKind::Loop(..) => "`loop`",
                        _ => "block",
                    };
                    let (start, end) = (e.span.start, e.span.end);
                    self.fits(&t, &Type::Unit, result_span(e), |t, _| {
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
        if let (false, Some(_), Some(tail)) = (warned, self.diverges, tail) {
            self.warnings.push(unreachable_code(tail.span));
        }
        let t = match (tail, want) {
            (Some(tail), Some(want)) => {
                self.check(tail, want)?;
                want.clone()
            }
            (Some(tail), None) => self.synth(tail)?,
            (None, _) if self.diverges.is_some() => Type::Never,
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
        let mut pats = Vec::new();
        let mut binds = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            pats.push(self.pattern(&arm.pat, &t, None, &mut b)?);
            self.note_binds(&b, true);
            b.iter().for_each(|l| self.bind(l));
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

        // No arms: `match n {}`, on a value that can't exist, so it's
        // never run. It's a `!`, the type with no values, as it should be,
        // and diverges, as all of its arms do.
        let mut result = match arms.is_empty() {
            true => Some(Type::Never),
            false => want.cloned(),
        };
        let before = self.diverges.take();
        let mut all_diverge = true;
        for (arm, b) in arms.iter().zip(binds) {
            let scope = self.locals.len();
            self.locals.extend(b);
            match (&result, want) {
                (Some(r), Some(_)) => self.check(&arm.body, &r.clone())?,
                (Some(r), None) => {
                    let r = r.clone();
                    let u = self.synth(&arm.body)?;
                    let joined = self.join(&r, &u, result_span(&arm.body), |r, u| {
                        format!("`match` arms have different types: `{r}` and `{u}`")
                    })?;
                    result = Some(joined);
                }
                (None, _) => result = Some(self.synth(&arm.body)?),
            }
            all_diverge &= self.diverges.take().is_some();
            self.end_scope(scope);
        }
        self.diverges = before.or(all_diverge.then_some(e.span));
        Ok(result.expect("an arm, or none and `!`"))
    }

    /// Checks that pattern `p` can match values of type `ty`, and adds the
    /// variables it binds to `binds`. This is the judgment `p : T ⊣ Δ` from
    /// T-Match, with `binds` as Δ.
    ///
    /// `by_ref` says how variables bind: by value, moving or copying what
    /// they're bound to (`None`), or by reference to it (`Some`, with whether
    /// it's `&mut`). They bind by value until the pattern looks through a
    /// reference: that's *match ergonomics*, as in Rust. In
    /// `match r { Option::Some(x) => .. }` with `r: &Option<int>`, the
    /// variant pattern looks through the `&` to the `Option<int>` behind it,
    /// and `x` is a `&int`, borrowed from inside it.
    ///
    /// Returns the pattern in the reduced form the exhaustiveness checker
    /// uses.
    fn pattern(
        &mut self,
        p: &Pattern,
        ty: &Type,
        mut by_ref: Option<bool>,
        binds: &mut Vec<Local>,
    ) -> Result<Pat, Error> {
        let looks_inside = !matches!(
            p.kind,
            PatternKind::Wild | PatternKind::Bind(..) | PatternKind::Or(_)
        );
        let mut ty = ty.clone();
        if looks_inside {
            let mut peeled = 0;
            while let Type::Ref(_, m, inner) = self.shallow(&ty) {
                // Through a `&` to a `&mut`, it's still only shared.
                by_ref = Some(by_ref.unwrap_or(true) && m);
                ty = *inner;
                peeled += 1;
            }
            if peeled > 0 {
                self.typed.peels.insert(p, peeled);
            }
        }
        let ty = &ty;
        Ok(match &p.kind {
            PatternKind::Wild => Pat::Wild,
            PatternKind::Bind(name, mutable) => {
                if binds.iter().any(|l| l.name == *name) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{name}` is bound more than once in the same pattern"),
                    ));
                }
                if *mutable && by_ref.is_some() {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "`{name}` can't be `mut` here: it's bound by reference, as the \
                             pattern looks through a reference to get to it"
                        ),
                    ));
                }
                let t = match by_ref {
                    Some(m) => Type::Ref(Region::Erased, m, Box::new(ty.clone())),
                    None => ty.clone(),
                };
                let mut local = Local::new(name, t, p.name_span());
                local.mutable = *mutable;
                local.mut_span = mutable.then(|| Span::new(p.span.start, p.name_span().start));
                local.by_ref = by_ref;
                binds.push(local);
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
                if let Type::Adt(symbol, ..) = &t {
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
                    .map(|(a, t)| self.pattern(a, t, by_ref, binds))
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
                    pats.push((self.pattern(alt, ty, by_ref, &mut b)?, alt.span));
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
                let Type::Adt(symbol, ..) = &t else {
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
                    args[i] = self.pattern(&fp.pat, &fields[i].1, by_ref, binds)?;
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
        let def = self.adt_symbol(name.span).and_then(|s| self.adts.get(s));
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
        let symbol = self.adt_symbol(name.span).expect("a struct or enum");
        let symbol = symbol.to_string();
        let params = self.adts[&symbol].params.clone();
        let args = self.type_args(&name.name, &params, targs, name.span, rest)?;
        let lifetimes = self.adts[&symbol].lifetimes.len();
        let ty = Type::Adt(
            symbol.clone(),
            vec![Region::Erased; lifetimes],
            args.clone(),
        );
        self.well_formed(&ty, name.span)?;
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
        let symbol = self.adt_symbol(ty.span).map(str::to_string);
        let def = symbol.as_ref().and_then(|s| self.adts.get(s));
        if def.is_none() && ty.name == "Box" {
            return Err(Error::ty(
                ty.span,
                "`Box` isn't an enum: `Box::new(x)` makes one, and `*b` is what box `b` holds",
            ));
        }
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

    /// Notes the use of method `found`, named at `name`, for editors and for
    /// monomorphisation. Its type arguments were `inferred` unless they
    /// were written.
    fn resolved(&mut self, name: &Ident, found: &traits::Found, inferred: bool) {
        let instance = Instance::Fn(found.symbol.clone(), found.args.clone());
        let kind = NoteKind::Fn(name.name.clone());
        let ty = found.ty.clone();
        self.note_use(
            name.span,
            kind,
            ty,
            found.def,
            Some((name.span, instance)),
            inferred,
        );
    }

    /// Is `ty::name` a variant of an enum, rather than a path to a method?
    fn is_variant(&self, ty: &Ident, name: &Ident) -> bool {
        let def = self.adt_symbol(ty.span).and_then(|s| self.adts.get(s));
        match def.map(|d| &d.kind) {
            Some(AdtKind::Enum(variants)) => variants.iter().any(|(v, _)| *v == name.name),
            // Not a method either, but the variant rule says what's wrong.
            _ => def.is_none() && self.path_type(ty).is_none(),
        }
    }

    /// What `ty` in a path `ty::m` names, if it's a type: `Self`, a type
    /// parameter, or `int` or `bool`. Structs and enums are instantiated by
    /// `path`, as they may be generic.
    fn path_type(&self, ty: &Ident) -> Option<Type> {
        match ty.name.as_str() {
            "Self" if self.self_ty.is_some() => self.self_ty.clone(),
            "int" => Some(Type::Int),
            "bool" => Some(Type::Bool),
            n if self.type_params.iter().any(|p| p == n) => Some(Type::Param(n.into())),
            n if self.traits.traits.contains_key(n) => Some(Type::Unit),
            _ => None,
        }
    }

    //   m is a method of T of type F
    //  ------------------------------ (T-Path)
    //          Γ ⊢ T::m : F
    //
    //   m is a method of trait Tr of type F, at Self = S    S: Tr
    //  ---------------------------------------------------------- (T-Path-Trait)
    //                    Γ ⊢ Tr::m : F
    //
    // A path names a method as a function, with its receiver, if it has
    // one, as its first parameter: `Point::len(&p)` is `p.len()`. It's
    // found as for a call, without going through references (see
    // `lookup_method`). In `Show::show(x)`, `Self` is worked out from how
    // it's used, here from `x`, as for any type parameter. With `(..)` it's
    // called (T-App).
    fn path(
        &mut self,
        e: &Expr,
        ty: &Ident,
        targs: &[TypeExpr],
        name: &Ident,
        args: &[Expr],
        parens: bool,
    ) -> Result<Type, Error> {
        let found = match self.traits.traits.contains_key(&ty.name)
            && self.adt_symbol(ty.span).is_none()
            && !self.type_params.contains(&ty.name)
        {
            true => {
                let tr = self.trait_named(ty)?;
                if let Some(t) = targs.first() {
                    return Err(Error::ty(
                        t.span,
                        format!("trait `{tr}` takes no type arguments; its methods may"),
                    ));
                }
                let what = format!("`Self` for this use of `{tr}::{}`", name.name);
                let hint = "annotate the type of its argument".to_string();
                let s = self.fresh(name.span, what, hint);
                self.lookup_method(&s, Some(&tr), name, &[], "trait")?
            }
            false => {
                let t = match self.adt_symbol(ty.span) {
                    Some(_) => {
                        let rest = format!("::{}", name.name);
                        let kind = match self.adts[self.adt_symbol(ty.span).unwrap()].kind {
                            AdtKind::Struct(_) => NoteKind::Struct,
                            AdtKind::Enum(_) => NoteKind::Enum,
                        };
                        self.instantiate(ty, targs, &rest, kind)?
                    }
                    None => {
                        let t = self.path_type(ty).expect("checked by `is_variant`");
                        if let Some(t) = targs.first() {
                            return Err(Error::ty(
                                t.span,
                                format!("`{}` takes no type arguments", ty.name),
                            ));
                        }
                        t
                    }
                };
                let what = match self.adt_symbol(ty.span).map(|s| &self.adts[s].kind) {
                    Some(AdtKind::Enum(_)) => "enum",
                    _ => "type",
                };
                self.lookup_method(&t, None, name, &[], what)
                    .map_err(|err| match what {
                        "enum" => Error::ty(
                            name.span,
                            format!("`{}` has no variant or method `{}`", ty.name, name.name),
                        ),
                        _ => err,
                    })?
            }
        };
        self.resolved(name, &found, targs.is_empty());
        self.typed
            .vars
            .insert(e, Res::Fn(found.symbol.clone(), found.args.clone()));
        let resolved = Resolved {
            symbol: found.symbol.clone(),
            derefs: 0,
            autoref: None,
        };
        self.info.methods.insert(name.span, resolved);
        if let Some(pred) = found.of_trait.clone() {
            self.oblige(pred, name.span, traits::Why::Method(name.name.clone()))?;
        }
        if !parens {
            // Without `(..)`, it's this whole expression that names it.
            let instance = Instance::Fn(found.symbol.clone(), found.args.clone());
            self.pending.push((e.span, instance));
            return Ok(found.ty);
        }
        let Type::Fn(sig) = &found.ty else {
            unreachable!("a method has a function type")
        };
        if sig.params.len() != args.len() {
            return Err(Error::ty(
                e.span,
                format!(
                    "`{}::{}` takes {} but {} {} supplied",
                    ty.name,
                    name.name,
                    plural(sig.params.len(), "argument"),
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" },
                ),
            ));
        }
        for (arg, param) in args.iter().zip(&sig.params) {
            self.check(arg, param)?;
        }
        Ok((*sig.ret).clone())
    }

    /// Does `name`, written as the type in `name::new(..)`, mean `Box`?
    fn is_box(&self, name: &Ident) -> bool {
        name.name == "Box" && self.adt_symbol(name.span).is_none()
    }

    //        Γ ⊢ e : T
    //  -------------------------- (T-Box)
    //   Γ ⊢ Box::new(e) : Box<T>
    //
    // Puts `e` on the heap: the one way a T program allocates memory.
    fn box_new(
        &mut self,
        e: &Expr,
        targs: &[TypeExpr],
        new: &Ident,
        args: &[Expr],
    ) -> Result<Type, Error> {
        if new.name != "new" {
            return Err(Error::ty(
                new.span,
                format!(
                    "`Box` has no `{}`; `Box::new(x)` puts `x` in a box",
                    new.name
                ),
            ));
        }
        let [arg] = args else {
            return Err(Error::ty(
                e.span,
                format!(
                    "`Box::new` takes 1 argument but {} were supplied",
                    args.len()
                ),
            ));
        };
        let t = match targs {
            [] => self.synth(arg)?,
            [t] => {
                let t = self.type_arg(t)?;
                self.check(arg, &t)?;
                t
            }
            _ => return Err(generics::arity_error("`Box`", 1, targs.len(), e.span)),
        };
        Ok(Type::Box(Box::new(t)))
    }
}

/// Where a statement is, if it's code, which runs, rather than a nested
/// function or type.
fn stmt_span(stmt: &Stmt) -> Option<Span> {
    match stmt {
        Stmt::Let { span, .. } => Some(*span),
        Stmt::Expr(e) | Stmt::BlockLike(e) => Some(e.span),
        Stmt::Fn(_) | Stmt::Type(_) => None,
    }
}

/// Where variable `name` is first used in `e`, if it is.
pub(crate) fn first_use(e: &Expr, name: &str) -> Option<Span> {
    match &e.kind {
        ExprKind::Var(n, _) if n == name => Some(e.span),
        _ => e.children().into_iter().find_map(|c| first_use(c, name)),
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
