//! The type checker. This is the simply typed lambda calculus (STLC) with base
//! types `i64`, `bool`, `()` and function types, plus `let`, `if` and
//! recursive functions (step 1), plus structs, enums and `match`
//! (step 2, see docs/02-adts.md), plus closures, nested functions and types,
//! and type inference (step 3, see docs/03-inference.md), plus generics
//! (step 4, see docs/04-generics.md), plus references, boxes, mutation and
//! moves (step 5, see docs/05-ownership.md), plus `return`, `break`,
//! `continue`, `loop` and the never type `!` (see
//! docs/05.1-control-flow.md), plus traits, impls and methods (step 6,
//! see docs/06-traits.md, and `check/traits.rs`), plus opaque types, `impl
//! Fn(i64) -> i64` in a return type (see docs/06c.1-opaque-types.md), plus
//! modules, resolved before anything else (step 6d, see
//! docs/06d-modules.md, and `check/modules.rs`).
//! Whether a program uses its
//! references and moved values correctly is checked after its types are,
//! by the borrow checker (`borrowck`), from what `typed` records here.
//!
//! Each case of `check` and `synth` (`check/expr.rs`, and the patterns,
//! paths and closures they reach) is an implementation of a *typing rule*,
//! written above it in the usual inference-rule notation:
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
//! Every rule is *syntax-directed*: for each form of expression
//! exactly one rule applies, so the rules can be read directly as a recursive
//! algorithm. That's not true of every type system (see docs/01-stlc.md).
//!
//! Where a rule needs a type the program doesn't write down, like the `T` of
//! the parameter in `|x| x + 1`, the checker makes up a *type variable* and
//! works out what it must be from how it's used: `unify`.
//!
//! This file has the passes over the program and the checker's state; the
//! rest is in `check/`:
//!
//! - `modules.rs`, `names.rs`, `scope.rs`: what names refer to
//! - `adts.rs`: structs and enums; `traits.rs` and `traits/`: traits and impls
//! - `generics.rs`: written types, type parameters, polymorphic recursion
//! - `opaque.rs`: `impl Trait` return types
//! - `expr.rs`, `patterns.rs`, `paths.rs`, `closures.rs`: the typing rules
//! - `unify.rs`, `effects.rs`: type variables and effect sets
//! - `info.rs`, `typed.rs`: what checking records for later passes
//! - `lints.rs`: warnings

mod adts;
mod closures;
mod effects;
mod expr;
mod generics;
mod infer;
mod info;
mod ints;
mod lints;
pub(crate) mod modules;
mod names;
mod opaque;
mod paths;
mod patterns;
mod refine;
mod scope;
mod traits;
pub(crate) mod typed;
mod unify;

pub(crate) use closures::is_result_lifetime;
pub(crate) use expr::{first_use, result_span};
pub(crate) use generics::{
    AliasDef, Kind, Lifetimes, ProjNames, TypeName, resolve, resolve_effects,
};
pub use info::{Info, Instance, MoveSite, Note, NoteKind, Test};
pub use scope::source_name;

use crate::ast::*;
use crate::error::{Error, Fix, Span, Warning};
use crate::traits::EQ;
use crate::traits::{Pred, Traits};
use crate::types::{Adts, Params, Region, Scheme, Type};
use adts::{adt_names, each_adt};
use expr::Target;
use info::Defs;
use lints::{Uses, unused, unused_doc};
use names::{Scoped, TypeDef};
use opaque::{opaques_in, opaques_of};
use scope::{FnDef, FnId, FnScope};
use std::collections::{HashMap, HashSet};
use typed::{Bind, Typed};
use unify::Var;

/// The intrinsics: functions the core and alloc libraries declare with no
/// body, which the compiler provides (see `core/mem.t`, `alloc/raw.t`), by
/// symbol, or for a method, its type's symbol and its name (`[T]` for a
/// slice's). Each is checked
/// like any function, by its signature; what it does is what the checker
/// trusts.
pub const INTRINSICS: &[&str] = &[
    "core::mem::swap",
    "core::panic::panic",
    "core::panic::Message::new",
    "core::panic::Message::push_str",
    "core::panic::Message::push_char",
    "core::panic::Message::abort",
    "core::num::checked_mul",
    "core::num::wrapping_mul",
    "core::num::wrap",
    "core::num::least",
    "core::num::greatest",
    "core::slice::empty",
    "core::str::from_utf8_unchecked",
    "str::as_bytes",
    "core::slice::empty_mut",
    "[T]::len",
    "str::len",
    "[T]::index",
    "[T]::index_mut",
    "[T]::get",
    "[T]::get_mut",
    "[T]::split_at",
    "[T]::split_at_mut",
    "[T; N]::len",
    "[T; N]::as_slice",
    "[T; N]::as_mut_slice",
    "core::array::from_fn",
    "alloc::raw::RawBuf::new",
    "alloc::raw::RawBuf::empty",
    "alloc::raw::RawBuf::cap",
    "alloc::raw::RawBuf::grow",
    "alloc::raw::RawBuf::write",
    "alloc::raw::RawBuf::read",
    "alloc::raw::RawBuf::slice",
    "alloc::raw::RawBuf::slice_mut",
    "alloc::raw::RawBuf::free",
    "alloc::raw::out_of_memory",
    "alloc::arena::Arena::new",
    "alloc::arena::Arena::alloc",
    "alloc::arena::Arena::len",
    "std::io::Stdin::read_byte",
    "std::io::Stdout::write_byte",
    "std::io::Stderr::write_byte",
];

/// The intrinsic structs, by symbol: types the alloc library declares
/// with no fields, which only the compiler makes and looks inside, by the
/// intrinsic methods of their impls.
pub const INTRINSIC_TYPES: &[&str] = &[
    "core::panic::Message",
    BOX,
    "alloc::raw::RawBuf",
    ARENA,
    "std::io::Stdin",
    "std::io::Stdout",
    "std::io::Stderr",
];

/// The symbol of the alloc library's `Arena`, the one intrinsic type
/// that's invariant in its parameter (lemma R3 of docs/23-arenas.md).
pub const ARENA: &str = "alloc::arena::Arena";

/// The symbol of the alloc library's `Box`, which is `Type::Box`.
pub const BOX: &str = "alloc::boxed::Box";

pub(crate) use modules::FOR_ITERATOR;

/// The symbol of the core library's `Iterator`, which a `for` loop uses.
pub(crate) const ITERATOR: &str = "core::iter::Iterator";

/// The const parameters `g` declares.
fn consts(g: &Generics) -> Vec<Ident> {
    g.types.iter().filter(|p| g.is_const(p)).cloned().collect()
}

/// That only the built-in libraries can declare an intrinsic, written at
/// `at`.
pub(super) fn not_library(at: Span) -> Error {
    Error::ty(
        at,
        "only `core`, `alloc` and `std` can declare an intrinsic: what it does is what the \
         checker trusts",
    )
}

/// What reading and checking the libraries built into the compiler left,
/// which every program's checking starts from (see docs/14-packages.md):
/// their files, their items, with their paths resolved, and an empty crate
/// root, and what the checker knows of them.
pub struct Libraries {
    pub sources: crate::source::Sources,
    pub prog: Program,
    /// Behind a lock only as it has cells in it, which each program's
    /// checking clones.
    cx: std::sync::Mutex<Checker>,
}

/// The libraries, read and checked, each against those before it: core
/// alone, then alloc. Only once, as they're the same for every program.
pub fn libraries() -> &'static Libraries {
    static LIBRARIES: std::sync::OnceLock<Libraries> = std::sync::OnceLock::new();
    LIBRARIES.get_or_init(|| {
        let (mut sources, mut prog) = crate::load::empty();
        let mut cx = Checker::new();
        for library in crate::load::LIBRARIES {
            let from = prog.counts();
            let checked = crate::load::library(&mut prog, &mut sources, library)
                .and_then(|root| cx.package(&mut prog, root, from));
            if let Err(e) = checked {
                panic!(
                    "the {} library doesn't check: {}",
                    library.0,
                    e.render_in(&sources)
                );
            }
        }
        // Its own business, as `check_files` says.
        cx.warnings.clear();
        Libraries {
            sources,
            prog,
            cx: std::sync::Mutex::new(cx),
        }
    })
}

/// Type-checks `prog`, adding any warnings to `warnings`, in source order.
/// Those found before an error are kept, and so is what was learned about
/// the program up to it. Resolving its paths first rewrites it, as
/// `check/modules.rs` says.
///
/// Its libraries are `libraries()`'s, as `load` reads them, so only the
/// program's packages are checked here, against what checking them left:
/// those it depends on, each against those before it, which are those it
/// depends on (see `load.rs`), then the crate root's.
pub fn check(prog: &mut Program, warnings: &mut Vec<Warning>) -> (Info, Result<(), Error>) {
    let libraries = libraries();
    let mut cx = libraries.cx.lock().expect("never poisoned").clone();
    let packages = match prog.packages.is_empty() {
        // A program put together by hand, not by `load`.
        true => vec![(ROOT, libraries.prog.counts())],
        false => prog.packages.iter().map(|p| (p.root, p.from)).collect(),
    };
    let mut result = Ok(());
    for (i, &(root, from)) in packages.iter().enumerate() {
        // Each package is read before those after it are, but checked as
        // if they weren't yet: its passes are over what's after `from`.
        let rest = packages.get(i + 1).map(|&(_, to)| prog.split_off(to));
        result = cx.package(prog, root, from);
        if let Some(rest) = rest {
            prog.append(rest);
        }
        if result.is_err() {
            break;
        }
    }
    if let Err(e) = &mut result {
        import_fix(prog, &cx.modules, e);
    }
    cx.info.types = cx.defs.types.clone();
    let lib_types = cx.types.iter().map(|t| (&t.symbol, t.module));
    let lib_fns = cx.fns.iter().map(|f| (&f.symbol, f.module));
    cx.info.library_symbols = lib_types
        .chain(lib_fns)
        .filter(|&(_, m)| cx.modules.in_library(m) || cx.modules.in_dependency(m))
        .map(|(s, _)| s.clone())
        .collect();
    cx.info.projections = cx.proj_traits.take();
    cx.info.moves = cx.moves.take();
    cx.info.mir = cx.mir.take();
    cx.warnings.sort_by_key(|w| w.span.start);
    warnings.append(&mut cx.warnings);
    prog.next_id = cx.next_id;
    (cx.info, result)
}

/// Adds to `e`, if it's that a type is unknown that a library has, a fix
/// that imports it, into the innermost module of the program around it:
/// `use alloc::boxed::Box;` for `Box`, which isn't in the prelude.
fn import_fix(prog: &Program, modules: &modules::Modules, e: &mut Error) {
    let Some(name) = (e.message.strip_prefix("unknown type `"))
        .and_then(|rest| rest.strip_suffix('`'))
        .map(str::to_string)
    else {
        return;
    };
    let Some(path) = modules.library_type(&name) else {
        return;
    };
    let around = |s: Span| s.start <= e.span.start && e.span.end <= s.end;
    let Some(m) = (0..prog.mods.len())
        .filter(|&m| !modules.in_library(m) && around(prog.mods[m].span))
        .min_by_key(|&m| prog.mods[m].span.end - prog.mods[m].span.start)
    else {
        return;
    };
    e.message
        .push_str(&format!("; import it with `use {path};`"));
    e.fixes.push(Fix::new(
        format!("import `{name}`"),
        vec![crate::error::Edit::insert(
            modules.ends[m],
            format!("\nuse {path};\n"),
        )],
    ));
}

/// Checks that there's a `main`, and that it takes at most one parameter,
/// which `Checker::main_signature` checks is the `std::Os`.
fn check_main(prog: &Program) -> Result<(), Error> {
    match prog
        .fns
        .iter()
        .find(|f| f.module == ROOT && f.name.name == "main")
    {
        None => {
            let start = prog.mods[ROOT].span.start;
            let at = Span::new(start, start);
            Err(Error::ty(at, "program has no `main` function"))
        }
        Some(main) if main.params.len() > 1 => Err(Error::ty(
            main.params[1].name.span,
            "`main` takes nothing, or the `std::Os`, as `fn main(os: Os)`",
        )),
        // Its obligations would be its caller's to keep, and nothing calls
        // it: they'd be kept by no one.
        Some(main)
            if let Some(e) = main
                .effects
                .iter()
                .flat_map(|e| &e.items)
                .find(|e| matches!(&e.kind, EffectKind::Named(n) if n.name == "unsafe")) =>
        {
            Err(Error::ty(
                e.span,
                "`main` can't say `with unsafe`: nothing calls it to keep its obligations",
            ))
        }
        Some(_) => Ok(()),
    }
}

/// A `#[test]` function, as checking its signature found it.
#[derive(Debug, Clone)]
struct TestFn {
    fn_id: FnId,
    symbol: String,
    /// Its name, where it's declared.
    span: Span,
    panics: bool,
    error: Option<Type>,
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
    /// The binding pattern, or the parameter's name.
    node: NodeId,
    shorthand: bool,
}

impl Local {
    fn new(name: &str, ty: Type, span: Span, node: NodeId) -> Local {
        Local {
            name: name.to_string(),
            ty,
            mutable: false,
            mut_span: None,
            by_ref: None,
            sites: vec![Site {
                span,
                node,
                shorthand: false,
            }],
            used: false,
        }
    }

    /// Its id: the node where its name is first bound (see `typed`).
    fn id(&self) -> NodeId {
        self.sites[0].node
    }

    /// Where its name is first bound.
    fn def(&self) -> Span {
        self.sites[0].span
    }

    /// A parameter.
    fn param(name: &Ident, ty: Type) -> Local {
        Local::new(&name.name, ty, name.span, name.id)
    }
}

/// "1 field", "2 fields".
fn plural(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

#[derive(Clone)]
pub(crate) struct Checker {
    /// `Program::next_id`, while checking: the nodes the checker makes up
    /// take ids from it, and the program's goes on from where it got to.
    next_id: u32,
    /// What the packages before the one being checked have, which it comes
    /// after: its own items are the rest (see `Counts`).
    from: Counts,
    /// Where its `use`s' names start in `modules.leaf_items`.
    leaves_from: usize,
    /// Where its paths' modules start in `modules.module_refs`.
    refs_from: usize,
    /// How many blocks have declared types so far (see `names`).
    blocks: usize,
    /// Where the package being checked has its impls in `traits.impls`:
    /// those before are the packages' before it, which were checked
    /// against everything they could be checked against.
    impls_from: usize,
    /// Where the package being checked starts in the sources: its spans
    /// are after, as each package is read after those before it.
    spans_from: usize,
    /// What every struct and enum is made of, by symbol.
    adts: Adts,
    /// How many lifetime and type arguments each struct and enum takes, by
    /// symbol.
    arities: HashMap<String, (usize, Vec<bool>)>,
    /// What each type alias stands for, by symbol.
    aliases: HashMap<String, std::sync::Arc<AliasDef>>,
    /// Every struct and enum, top-level and local, in the order of
    /// `Program::all_types`.
    types: Vec<TypeDef>,
    /// What each name in a type refers to, by its node (see `names.rs`).
    type_names: HashMap<NodeId, Scoped>,
    /// The symbol of every function, by `FnId`.
    fn_symbols: Vec<String>,
    /// Every function, top-level and nested, by `FnId`.
    fns: Vec<FnDef>,
    /// Each nested function's id, by the node of its name.
    nested_ids: HashMap<NodeId, FnId>,
    /// The functions in scope, as a stack: those of each block around what's
    /// being checked that declares any. With those of its module and the
    /// built-ins, the global part of the context Γ (see `scope.rs`).
    scopes: Vec<FnScope>,
    /// Which item each path names, and what's in scope in each module (see
    /// `modules.rs`).
    modules: modules::Modules,
    /// The module of what's being checked.
    module: usize,
    /// The function being checked.
    current: Option<FnId>,
    /// The type parameters of the function being checked.
    type_params: crate::types::Params,
    /// Its const parameters, and its impl's, as declared: what a use of
    /// one as a value refers to.
    const_decls: Vec<Ident>,
    /// Its lifetime parameters, elided ones too.
    lifetimes: Vec<String>,
    /// What the borrow checker needs to know about each body.
    typed: Typed,
    /// The closures in the function being checked whose captures, or the
    /// lifetimes of whose results, are worked out at its end (see
    /// `closures`), innermost first.
    closures: Vec<closures::Pending>,
    /// What each lifetime of a closure's result that's worked out at the
    /// end of its function turned out to be, by its name (see
    /// `closures::result_lifetime`), for closures that are function types.
    decided: HashMap<(NodeId, usize), Region>,
    /// Why each closure implements only the `Fn` traits it does, by
    /// symbol, for messages: "it moves `b` out".
    closure_why: HashMap<String, String>,
    /// The calls in the function being checked of what isn't a function
    /// type, with the callee's type, and where: how each gets at what it
    /// calls is worked out at its end (`Checker::settle_calls`).
    calls: Vec<(NodeId, Type, Span)>,
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
    /// Each method's id, by the node of its name.
    method_ids: HashMap<NodeId, FnId>,
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
    /// arguments that may still have variables in them, and where each
    /// is. `finish_notes` moves them to `info`, and the spans to
    /// `instance_at`.
    pending: Vec<(NodeId, Span, Instance)>,
    /// The comparisons `<`, `<=`, `>` and `>=` in the function being
    /// checked, by node, with the type of what they compare: `Ord` calls,
    /// unless that turns out to be an integer type.
    compares: HashMap<NodeId, Type>,
    /// Where each use in `info.instances` is, by its node: what function
    /// it's in is worked out from that (`no_polymorphic_recursion`).
    instance_at: HashMap<NodeId, Span>,
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
    /// The trait of each projection written, by its node, as it's worked
    /// out while resolving types (see `Info::projections`).
    proj_traits: std::cell::RefCell<HashMap<NodeId, String>>,
    /// Where the borrow checker moved, as it checks each body, which only
    /// needs a `&Checker` (see `Info::moves`).
    pub(crate) moves: std::cell::RefCell<HashSet<MoveSite>>,
    /// Each body's MIR, once the borrow checker has checked it, for
    /// running (see `Info::mir`).
    pub(crate) mir: std::cell::RefCell<HashMap<crate::mir::Key, std::sync::Arc<crate::mir::Body>>>,
    /// Whether impls are being collected: then what a projection on a type
    /// that isn't a parameter stands for isn't known yet, nor whether the
    /// type implements the trait, which `check_impls` sees to.
    collecting: bool,
    /// Where the expression being checked is, for the goals it makes.
    here: Span,
    /// Projections on types not known yet, each waiting for its type to
    /// be, with what it must turn out to be (see `settle`).
    goals: Vec<Goal>,
    /// The symbol of each opaque type, `impl Tr` in a return type, by the
    /// node it's written as.
    opaque_at: HashMap<NodeId, String>,
    /// The function whose signature is being worked out, by symbol and
    /// name: what owns the opaque types in it.
    sig_owner: (String, String),
    /// The type of the impl whose method's signature is being checked, if
    /// it's one: what `Self` is in it.
    sig_self: Option<Type>,
    /// What has effects in the body being checked, the function's or a
    /// closure's (see `effects`).
    effect_sites: Vec<effects::Site>,
    /// Why an operator could not be proved total, at its effect site.
    proof_failures: HashMap<Span, refine::ProofFailure>,
    /// Calls of `Result`'s methods for an `Err`, with the error type, to
    /// see once it's solved whether there can be one (see
    /// `lints::unwrap_infallible`).
    unwraps: Vec<(lints::Unwrap, Type)>,
    /// Statements `e;`, with the type of `e`, to see once it's solved
    /// whether an error is thrown away with it (see
    /// `lints::unused_results`).
    dropped: Vec<(Span, Type)>,
    /// What must hold of effect sets once the function's variables are
    /// solved.
    withins: Vec<effects::Within>,
    /// The `unsafe` blocks in the body being checked.
    unsafe_blocks: Vec<Span>,
    /// The effects of each site in an `unsafe` block, by the block, to
    /// tell which have something unsafe in them.
    unsafe_uses: Vec<(Span, Type)>,
    /// Where the closure is whose effects each effect variable is, by its
    /// number, for messages.
    effect_owners: HashMap<u32, Span>,
    /// The variables that stand for effect sets.
    effect_vars: HashSet<u32>,
    /// Is the function being checked a method of an impl of a trait?
    in_trait_impl: bool,
    /// Is the body being checked an impl's own `discard`? Then `self`, at
    /// its end, is dropped part by part (lemma D3 of
    /// docs/20-destructors.md).
    pub(crate) in_discard: bool,
    /// The refinements in each function's signature that has any, by
    /// symbol (see `refine`).
    refined: HashMap<String, refine::RefinedSig>,
    /// The refinements of each struct's fields that has any, by symbol.
    field_refined: HashMap<String, refine::RefinedFields>,
    /// The refinement of each refined `let`'s variable, by id.
    refined_lets: HashMap<NodeId, refine::Refinement>,
    /// What each name in a refinement's predicate is, by node.
    pred_names: HashMap<NodeId, refine::PredName>,
    /// The measures, by symbol (see docs/21-measures.md).
    measures: HashMap<String, refine::Measure>,
    /// The measure each `x.m()` in a refinement's predicate is, by node,
    /// with how many `*`s it's through.
    pred_measures: HashMap<NodeId, (String, usize)>,
    /// What the refinement being checked is of, if it's checked more than
    /// once, so it can't name what's behind a `&mut`, which can change in
    /// between: "a result", say.
    pred_again: Option<&'static str>,
    /// In a postcondition (`ensures`, or a result's refinement), which
    /// it is, and which parameters are call-scoped `&mut`s, whose values
    /// at return it can name (see `refine::call_scoped`).
    pred_post: Option<(&'static str, Vec<bool>)>,
    /// Inside `old(..)` in a postcondition.
    pred_in_old: bool,
    /// The refinements to check once every impl is collected.
    pending_refined: Vec<refine::Pending>,
    /// The function whose refinement is being checked, which uses the
    /// measures it names; `None` for a struct's.
    pred_owner: Option<FnId>,
    /// The measures structs' refinements name, which aren't dead code.
    kept_measures: Vec<FnId>,
    /// The integer literals of the function being checked, in expressions
    /// and patterns, each to be checked to fit in its type once that's
    /// known (`Checker::literals`).
    literals: Vec<Literal>,
    /// The type of what each `!` of the function works on, with where it
    /// is, of those not known when it was checked (`Checker::nots`).
    nots: Vec<(Type, Span)>,
    /// The functions whose effects are being inferred, by `FnId`, whose
    /// group's bodies aren't all checked yet (see `check/infer.rs`).
    inferring: HashSet<usize>,
    /// Every package's tests so far, and where the one being checked's
    /// start.
    tests: Vec<TestFn>,
    tests_from: usize,
    /// The sites of the body of each of them that's checked, kept for
    /// working out its effects with its group's.
    infer_sites: HashMap<usize, Vec<effects::Site>>,
    /// The functions of groups whose effects wait for another's.
    waiting: Vec<FnId>,
    /// What waits for effects being inferred to be known.
    deferred: Vec<effects::Deferred>,
    /// The effects inferred for each function, by symbol, at its own
    /// parameters.
    inferred: HashMap<String, Type>,
    /// Why each inferred function has each of its effects.
    witnesses: HashMap<(String, Type), infer::Witness>,
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

/// An integer literal, `value`, written at `span`, of type `ty`: in an
/// expression or a pattern. `-` of one is one too, so that the least `i8`
/// can be written `-128`.
#[derive(Clone)]
pub(super) struct Literal {
    node: NodeId,
    span: Span,
    value: i128,
    ty: Type,
}

/// The symbol of the std library's `Os`, which `main` can take.
pub(crate) const OS: &str = "std::Os";

/// The symbol of the core library's `Display`, which the error `main` gives
/// back must implement, to be reported.
pub(crate) const DISPLAY: &str = "core::fmt::Display";

/// The symbol of the core library's `Result`.
pub(crate) const RESULT: &str = "core::result::Result";

impl Checker {
    /// Checks what `main`, `f`, of type `sig`, takes and gives back. It
    /// takes nothing, or the `std::Os`, which the program starts with: so
    /// every stream the program can use descends from that one value (see
    /// docs/19-std-io.md). It gives back `()`, or a `Result<(), E>` whose
    /// error is reported, so it's `Display`, and then thrown away, so it's
    /// `Discard`.
    /// Gives back the error type, if it's a `Result`.
    fn main_signature(
        &self,
        f: &FnDecl,
        sig: &crate::types::FnType,
    ) -> Result<Option<Type>, Error> {
        if let ([p], [t]) = (f.params.as_slice(), sig.params.as_slice())
            && !matches!(t, Type::Adt(symbol, ..) if symbol == OS)
        {
            return Err(Error::ty(
                p.ty.span,
                format!("`main` takes nothing, or the `std::Os`, as `fn main(os: Os)`, not `{t}`"),
            ));
        }
        self.entry_result(f, sig, "`main`")
    }

    /// Checks what `f`, of type `sig`, gives back, as a program's start:
    /// `main`, or a test, `what`. It gives back `()`, or a `Result<(), E>`
    /// whose error is reported, so it's `Display`, and then thrown away,
    /// so it's `Discard`. Gives back the error type, if it's a `Result`.
    fn entry_result(
        &self,
        f: &FnDecl,
        sig: &crate::types::FnType,
        what: &str,
    ) -> Result<Option<Type>, Error> {
        let error = match &*sig.ret {
            Type::Unit => return Ok(None),
            Type::Adt(symbol, _, args) if symbol == RESULT && args[0] == Type::Unit => &args[1],
            t => {
                return Err(Error::ty(
                    f.ret.span,
                    format!(
                        "{what} gives back `()`, or a `Result<(), E>` whose error is reported, \
                         not `{t}`"
                    ),
                ));
            }
        };
        let display = self
            .traits
            .implements(error, DISPLAY, &self.env, &self.adts.proj.local);
        if display.is_err() {
            return Err(Error::ty(
                f.ret.span,
                format!(
                    "the error {what} gives back is written out, so it must be `Display`, and a \
                     `{error}` isn't"
                ),
            ));
        }
        if !self.is_discard(error) {
            return Err(Error::ty(
                f.ret.span,
                format!(
                    "the error {what} gives back is thrown away once it's written out, so it \
                     must be `Discard`, and a `{error}` isn't"
                ),
            ));
        }
        Ok(Some(error.clone()))
    }

    /// Checks test `f`, of type `sig`: it's run as a program of its own,
    /// as `main` is, so it's what `main` can be, but for the `Os`: tests
    /// can't do I/O, so what they do is all in what they give back
    /// (docs/31-tests.md, T2). Gives back its error type, if it gives a
    /// `Result`.
    fn test_signature(
        &self,
        f: &FnDecl,
        sig: &crate::types::FnType,
    ) -> Result<Option<Type>, Error> {
        if let Some(p) = f.params.first() {
            return Err(Error::ty(
                p.name.span,
                "a test takes nothing: it's run as a program of its own, and nothing could give \
                 it what it takes",
            ));
        }
        let g = &f.generics;
        if let Some(p) = g.lifetimes.first().or(g.types.first()) {
            return Err(Error::ty(
                p.span,
                "a test has no generic parameters: it's run as it is, and nothing could say what \
                 they are",
            ));
        }
        let unsafe_effect = (f.effects.iter().flat_map(|e| &e.items))
            .find(|e| matches!(&e.kind, EffectKind::Named(n) if n.name == "unsafe"));
        if let Some(e) = unsafe_effect {
            return Err(Error::ty(
                e.span,
                "a test can't say `with unsafe`: nothing calls it to keep its obligations",
            ));
        }
        self.entry_result(f, sig, "a test")
    }

    /// Checks that each test of the package just checked can pass, now
    /// that its effects are known: one that passes only by panicking must
    /// be able to (docs/31-tests.md, T3). The program's own are what `t
    /// test` runs.
    fn tests_known(&mut self, root: usize) -> Result<(), Error> {
        for t in &self.tests[self.tests_from..] {
            let Type::Fn(sig) = &self.fns[t.fn_id.0].scheme.ty else {
                continue;
            };
            let effects = self.with_inferred(&sig.effects);
            if t.panics && !effects.effect_items().contains(&Type::Panic) {
                return Err(Error::ty(
                    t.span,
                    "this test passes only if it panics, and it can't: nothing it does has the \
                     effect `panic`",
                ));
            }
        }
        if root == ROOT {
            self.info.tests = (self.tests[self.tests_from..].iter())
                .map(|t| info::Test {
                    symbol: t.symbol.clone(),
                    span: t.span,
                    panics: t.panics,
                    error: t.error.clone(),
                })
                .collect();
        }
        self.tests_from = self.tests.len();
        Ok(())
    }

    /// An id for a node the checker makes up (see `NodeId`).
    fn fresh_id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    /// A checker that knows of nothing but what's built in.
    fn new() -> Checker {
        Checker {
            adts: Adts::new(),
            arities: HashMap::new(),
            aliases: HashMap::new(),
            defs: Defs::default(),
            types: Vec::new(),
            type_names: HashMap::new(),
            fn_symbols: Vec::new(),
            blocks: 0,
            impls_from: 0,
            spans_from: 0,
            from: Counts::default(),
            leaves_from: 0,
            refs_from: 0,
            fns: Vec::new(),
            nested_ids: HashMap::new(),
            scopes: Vec::new(),
            current: None,
            type_params: Params::default(),
            const_decls: Vec::new(),
            lifetimes: Vec::new(),
            next_id: 0,
            typed: Typed::default(),
            closures: Vec::new(),
            decided: HashMap::new(),
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
            info: Info::default(),
            pending: Vec::new(),
            compares: HashMap::new(),
            instance_at: HashMap::new(),
            ret: None,
            loops: Vec::new(),
            diverges: None,
            diverging: Vec::new(),
            proj_traits: Default::default(),
            moves: Default::default(),
            mir: Default::default(),
            collecting: false,
            here: Span::new(0, 0),
            goals: Vec::new(),
            opaque_at: HashMap::new(),
            sig_owner: Default::default(),
            sig_self: None,
            effect_sites: Vec::new(),
            proof_failures: HashMap::new(),
            unwraps: Vec::new(),
            dropped: Vec::new(),
            withins: Vec::new(),
            unsafe_blocks: Vec::new(),
            unsafe_uses: Vec::new(),
            effect_owners: HashMap::new(),
            effect_vars: HashSet::new(),
            in_trait_impl: false,
            in_discard: false,
            refined: HashMap::new(),
            refined_lets: HashMap::new(),
            field_refined: HashMap::new(),
            pred_names: HashMap::new(),
            measures: HashMap::new(),
            pred_measures: HashMap::new(),
            pred_again: None,
            pred_post: None,
            pred_in_old: false,
            pending_refined: Vec::new(),
            pred_owner: None,
            kept_measures: Vec::new(),
            literals: Vec::new(),
            nots: Vec::new(),
            inferring: HashSet::new(),
            tests: Vec::new(),
            tests_from: 0,
            infer_sites: HashMap::new(),
            waiting: Vec::new(),
            deferred: Vec::new(),
            inferred: HashMap::new(),
            witnesses: HashMap::new(),
            modules: modules::Modules::default(),
            module: ROOT,
        }
    }

    /// Checks the package of `prog` whose root module is `root`, and whose
    /// items are those after `from` (see `Counts`), against what checking
    /// those before it left. Resolving its paths first rewrites them, as
    /// `check/modules.rs` says.
    fn package(&mut self, prog: &mut Program, root: usize, from: Counts) -> Result<(), Error> {
        self.from = from;
        self.leaves_from = self.modules.leaf_items.len();
        self.refs_from = self.modules.module_refs.len();
        self.impls_from = self.traits.impls.len();
        self.spans_from = prog.mods[root].span.start;
        let resolved = modules::resolve(prog, &mut self.modules, from);
        self.next_id = prog.next_id;
        resolved?;
        let resolved = std::mem::take(&mut self.modules.warnings);
        self.warnings.extend(resolved);
        let stray = &prog.stray_docs[from.stray_docs..];
        self.warnings.extend(stray.iter().map(unused_doc));
        self.fn_symbols = scope::symbols(prog, &self.modules, &self.fn_symbols);
        let (types, type_names) = names::names(
            prog,
            &prog.all_fns(),
            &self.fn_symbols,
            &self.modules,
            from,
            &self.types,
            &mut self.blocks,
        );
        for (id, name) in &type_names {
            if let Scoped::Adt(symbol) = name {
                self.info.symbols.insert(*id, symbol.clone());
            }
        }
        self.defs.add(prog, &types, from);
        self.types.extend(types);
        self.type_names.extend(type_names);
        let result = self.program(prog, root);
        self.type_notes(prog);
        result
    }

    fn program(&mut self, prog: &Program, root: usize) -> Result<(), Error> {
        self.trait_names(prog)?;
        self.collect_adts(prog)?;
        self.collect_traits(prog)?;
        self.check_aliases(prog)?;
        let fns = prog.all_fns();
        let own = fns.iter().enumerate().skip(self.from.all_fns);
        for (i, (f, ..)) in own.filter(|(_, (.., m))| m.is_some()) {
            self.method_ids.insert(f.name.id, FnId(i));
        }
        self.collect_impls(prog)?;
        self.projections_known(prog);
        self.derive(prog)?;
        self.signatures(prog, &fns)?;
        self.pending_refinements()?;
        self.check_impls(prog)?;
        self.info.traits = self.traits.clone();
        self.bodies(prog)?;
        self.zonk_inferred();
        self.tests_known(root)?;
        // What an editor can offer to write, of the program's.
        if root == ROOT {
            for (i, (f, _, method)) in prog
                .all_fns()
                .into_iter()
                .enumerate()
                .skip(self.from.all_fns)
            {
                if infer::infers(prog, f, method)
                    && let Some(t) = self.inferred.get(&self.fns[i].symbol)
                {
                    let fix = effects::inferred_fix(f, t);
                    self.info.inferred_fixes.push((f.name.span, fix));
                }
            }
        }
        // What each closure is is known once the body it's in is, and so
        // what each opaque type stands for.
        self.info.traits.closures = self.traits.closures.clone();
        self.opaque_cycles()?;
        self.info.traits.opaques = self.traits.opaques.clone();
        self.info.opaques = self.opaque_at.clone();
        self.no_polymorphic_recursion()?;
        // A library has no `main`, and nothing in it is dead that a program
        // could use.
        if root == ROOT {
            if !prog.is_library() {
                check_main(prog)?;
            }
            // What's dead depends on the whole program, so only once it all
            // checks.
            self.dead_code(prog);
        }
        let unused = self.modules.imports[self.leaves_from..]
            .iter()
            .filter(|i| i.level == Level::Module && !i.used.get());
        self.warnings.extend(unused.map(lints::unused_import));
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
        fns: &[(&FnDecl, Option<usize>, Option<MethodOf>)],
    ) -> Result<(), Error> {
        for (i, &(f, parent, method)) in fns.iter().enumerate().skip(self.from.all_fns) {
            let parent = parent.map(FnId);
            if parent.is_none()
                && method.is_none()
                && f.module == ROOT
                && f.name.name == "main"
                && !f.generics.types.is_empty()
            {
                return Err(Error::ty(
                    f.generics.types[0].span,
                    "`main` must not have type parameters: nothing could say what they are",
                ));
            }
            if let Some(at) = f.intrinsic {
                if !self.modules.in_library(f.module) {
                    return Err(not_library(at));
                }
                // A method's is its type's: `alloc::raw::RawBuf::new`.
                let name = match method {
                    Some(MethodOf::Impl(k)) => match &self.traits.impls[self.impl_ids[k]].self_ty {
                        Type::Adt(symbol, ..) => format!("{symbol}::{}", f.name.name),
                        Type::Slice(_) => format!("[T]::{}", f.name.name),
                        Type::Array(..) => format!("[T; N]::{}", f.name.name),
                        t => format!("{t}::{}", f.name.name),
                    },
                    _ => self.fn_symbols[i].clone(),
                };
                if !INTRINSICS.contains(&name.as_str()) {
                    return Err(Error::ty(
                        f.name.span,
                        format!("there's no intrinsic `{name}`"),
                    ));
                }
            }
            self.sig_owner = (self.fn_symbols[i].clone(), f.name.name.clone());
            self.const_decls = match method {
                Some(MethodOf::Impl(k)) => consts(&prog.impls[k].generics),
                _ => Vec::new(),
            };
            let scheme = match method {
                Some(MethodOf::Impl(k)) => {
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
                Some(MethodOf::Trait(k)) => {
                    let tr = &prog.traits[k].name.name;
                    let m = self.traits.method(tr, &f.name.name).expect("collected");
                    m.scheme.clone()
                }
                None => self.fn_scheme(f)?,
            };
            // Effects it doesn't say are its body's, which are inferred
            // with it.
            let mut scheme = scheme;
            if infer::infers(prog, f, method) {
                let effects = self.placeholder(&self.fn_symbols[i], &scheme.params);
                if let Type::Fn(sig) = &mut scheme.ty {
                    *sig.effects = effects;
                }
                self.inferring.insert(i);
            }
            if parent.is_none()
                && method.is_none()
                && f.module == ROOT
                && f.name.name == "main"
                && let Type::Fn(sig) = &scheme.ty
            {
                self.info.main_error = self.main_signature(f, sig)?;
            }
            if let Some(test) = f.test
                && let Type::Fn(sig) = &scheme.ty
            {
                let error = self.test_signature(f, sig)?;
                self.tests.push(TestFn {
                    fn_id: FnId(i),
                    symbol: self.fn_symbols[i].clone(),
                    span: f.name.span,
                    panics: test.panics,
                    error,
                });
            }
            let symbol = self.fn_symbols[i].clone();
            self.receivers.push(f.receiver().is_some());
            // Two in a module of the same name are found by resolving names
            // (see `modules.rs`); nested ones, by scope.
            if parent.is_some() {
                self.nested_ids.insert(f.name.id, FnId(i));
            }
            self.info.symbols.insert(f.name.id, symbol.clone());
            self.fns.push(FnDef {
                name: f.name.clone(),
                symbol,
                scheme,
                parent,
                body: f.body.span,
                module: f.module,
                level: f.vis.level(),
            });
        }
        self.uses.resize_with(fns.len(), Uses::default);
        Ok(())
    }

    /// The type of function `f`, which isn't a method.
    fn fn_scheme(&mut self, f: &FnDecl) -> Result<Scheme, Error> {
        let params = generics::declare(&f.generics)?;
        let declared = generics::declare_lifetimes(&f.generics.lifetimes)?;
        let bounds = generics::bounds(&f.generics, &declared, &params)?;
        let (preds, eqs) = self.preds(&f.generics, &params, &declared, true)?;
        self.env = preds.clone();
        self.adts.proj.local = eqs.clone();
        let (param_tys, ret, effects, lifetimes) = self.signature(f, &params, &declared, true)?;
        self.env.clear();
        self.adts.proj.local.clear();
        Ok(Scheme {
            params,
            lifetimes,
            bounds,
            preds,
            eqs,
            ty: Type::func_with(param_tys, ret, effects),
        })
    }

    /// The parameter and return types of `f`, which has type and effect
    /// parameters `params` and declares lifetimes `declared`, its effects,
    /// and its lifetime parameters: those, and one more for each lifetime
    /// its parameters leave out.
    ///
    /// That's *lifetime elision*, as in Rust. A lifetime its return type
    /// leaves out is the one lifetime its parameters have, if they have
    /// exactly one: `fn first(p: &Pair) -> &i64` returns something borrowed
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
        params: &Params,
        declared: &[String],
        opaques: bool,
    ) -> Result<(Vec<Type>, Type, Type, Vec<String>), Error> {
        let mut lts = Lifetimes::Params {
            declared,
            elided: Vec::new(),
        };
        let param_tys = f
            .params
            .iter()
            .map(|p| self.resolve_with(p.ty.unrefined(), params, &mut lts))
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
        let own = params.without("Self");
        for t in written {
            self.opaque(t, &own, &lifetimes)?;
        }
        let mut lts = Lifetimes::Return { declared, input };
        let ret = self.resolve_with(f.ret.unrefined(), params, &mut lts)?;
        self.refined_signature(f, &param_tys, &ret, opaques)?;
        let effects = match &f.effects {
            Some(e) => {
                let mut lts = Lifetimes::Assoc {
                    declared: &lifetimes,
                };
                self.resolve_effects_with(e, params, &mut lts)?
            }
            None => Type::no_effects(),
        };
        Ok((param_tys, ret, effects, lifetimes))
    }

    /// Pass 5: checks each function's body against its declared return
    /// type. The methods' too. Nested functions are checked with the
    /// bodies they're in.
    fn bodies(&mut self, prog: &Program) -> Result<(), Error> {
        // Measures first, so that what each is, its body's terms, is known
        // where another body uses it. Then callees first, a group at a
        // time, so that what's inferred of a function's effects is known
        // where it's called (see `check/infer.rs`).
        let all = prog.all_fns();
        let roots: Vec<usize> = (0..all.len()).map(|i| infer::root(&all, i)).collect();
        for group in infer::body_groups(prog, &all, self.from.all_fns) {
            for &i in &group {
                self.fn_body(prog, all[i], i)?;
            }
            let members = (self.from.all_fns..all.len())
                .filter(|&j| group.contains(&roots[j]) && self.inferring.contains(&j))
                .map(FnId)
                .collect();
            self.solve_group(members, false)?;
        }
        self.solve_group(Vec::new(), true)?;
        self.inferring.clear();
        Ok(())
    }

    /// Checks the body of function `i` of `Program::all_fns`, `f`, with
    /// the ones nested in it.
    fn fn_body(
        &mut self,
        prog: &Program,
        (f, parent, method): (&FnDecl, Option<usize>, Option<MethodOf>),
        i: usize,
    ) -> Result<(), Error> {
        {
            // What an intrinsic does isn't written in T.
            if f.intrinsic.is_some() || parent.is_some() {
                return Ok(());
            }
            self.self_ty = match method {
                Some(MethodOf::Impl(k)) => {
                    Some(self.traits.impls[self.impl_ids[k]].self_ty.clone())
                }
                _ => None,
            };
            self.in_trait_impl = matches!(
                method,
                Some(MethodOf::Impl(k)) if prog.impls[k].trait_name.is_some()
            );
            self.in_discard = matches!(
                method,
                Some(MethodOf::Impl(k))
                    if self.traits.impls[self.impl_ids[k]].trait_name.as_deref()
                        == Some(crate::traits::DISCARD)
            );
            self.const_decls = match method {
                Some(MethodOf::Impl(k)) => consts(&prog.impls[k].generics),
                _ => Vec::new(),
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
        self.module = f.module;
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
        self.const_decls.extend(consts(&f.generics));
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
            let mut local = Local::param(&p.name, ty);
            local.mutable = p.mutable.is_some();
            local.mut_span = p.mutable.map(|m| Span::new(m.start, p.name.span.start));
            self.note_binds(std::slice::from_ref(&local), false);
            self.bind(&local);
            self.locals.push(local);
        }
        // A parameter a refinement names is used: by what it says.
        for i in self.named_params(&self.fns[id.0].symbol) {
            self.locals[scope + i].used = true;
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
        self.solve_all(vars, f)?;
        for (call, err) in std::mem::take(&mut self.unwraps) {
            if self.solved(&err) == Type::Never {
                self.warnings.push(lints::unwrap_infallible(&call));
            }
        }
        // One that isn't `Discard` can't be thrown away at all, which is
        // an error of its own.
        for (at, t) in std::mem::take(&mut self.dropped) {
            let t = self.solved(&t);
            if let Type::Adt(symbol, _, args) = &t
                && symbol == RESULT
                && args.get(1).is_some_and(|e| *e != Type::Never)
                && self.is_discard(&t)
            {
                self.warnings.push(lints::unused_results(at));
            }
        }
        self.body_effects(f, &sig.effects)?;
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
        let symbol = self.fns[id.0].symbol.clone();
        let warnings = crate::borrowck::check(self, f, &symbol, &scheme)?;
        self.warnings.extend(warnings);
        Ok(())
    }

    /// What every struct and enum is made of, by symbol.
    /// Checks that `field` of the struct with symbol `symbol` can be seen
    /// here, as its `pub` or `export` says.
    fn field_visible(&self, symbol: &str, field: &Ident) -> Result<(), Error> {
        let Some(def) = self.types.iter().find(|d| d.symbol == symbol) else {
            return Ok(());
        };
        let Some(&(_, level)) = def.fields.iter().find(|(name, _)| *name == field.name) else {
            return Ok(());
        };
        if self.modules.visible(def.module, level, self.module) {
            return Ok(());
        }
        Err(Error::ty(
            field.span,
            format!(
                "field `{}` of `{}` {}",
                field.name,
                source_name(symbol),
                self.modules.hidden_by(def.module, level)
            ),
        ))
    }

    /// Checks that the struct with symbol `symbol`, written at `at` in a
    /// literal or pattern, isn't intrinsic: what's in one is the
    /// compiler's, so a literal would be one with nothing behind it, and a
    /// pattern would use one up without its methods.
    fn not_intrinsic(&self, symbol: &str, at: Span) -> Result<(), Error> {
        match self.types.iter().find(|d| d.symbol == symbol) {
            Some(def) if def.intrinsic.is_some() => Err(Error::ty(
                at,
                format!(
                    "`{}` is intrinsic: only its own functions make one or take one apart",
                    source_name(symbol)
                ),
            )),
            _ => Ok(()),
        }
    }

    /// Is the struct with symbol `symbol` abstract here: does it have
    /// fields that can't be seen? Then code here can only take it apart if
    /// its module says that's fine, by making it `Discard` (see
    /// `moves::abstraction`).
    pub(crate) fn is_abstract(&self, symbol: &str) -> bool {
        self.types.iter().any(|d| {
            d.symbol == symbol
                && (d.fields.iter()).any(|&(_, l)| !self.modules.visible(d.module, l, self.module))
        })
    }

    /// Is `span` a `?`'s, so that what's there is one of the calls it means
    /// (see `modules::Rewrite::try_expr`)?
    pub(crate) fn is_try(&self, span: Span) -> bool {
        self.modules.tries.contains(&span)
    }

    /// Is `span` where the `else` arm of what an `if let`, `while let` or
    /// `let .. else` means is, and if so, of which (see
    /// `modules::Rewrite::if_let`)?
    pub(crate) fn else_of(&self, span: Span) -> Option<modules::ElseOf> {
        self.modules.elses.get(&span).copied()
    }

    /// What a `return` here returns from, for messages: the innermost
    /// closure, or else the function.
    pub(crate) fn returning_from(&self) -> String {
        match (
            self.loops
                .iter()
                .any(|t| matches!(t, expr::Target::Closure)),
            self.current,
        ) {
            (false, Some(f)) => format!("`{}`", self.fns[f.0].name.name),
            _ => "this closure".to_string(),
        }
    }

    /// Is `t` a struct or enum of a library built into the compiler, which
    /// a program can't change?
    pub(crate) fn is_library_type(&self, t: &Type) -> bool {
        let Type::Adt(symbol, ..) = t else {
            return false;
        };
        (self.types.iter()).any(|d| d.symbol == *symbol && self.modules.in_library(d.module))
    }

    /// The module of the struct or enum with symbol `symbol`, for messages.
    /// Is the code being checked in the module that declares struct or
    /// enum `symbol`, or one inside it, where its private fields are seen?
    pub(crate) fn within_module_of(&self, symbol: &str) -> bool {
        self.types
            .iter()
            .any(|d| d.symbol == symbol && self.modules.within(self.module, d.module))
    }

    pub(crate) fn module_of(&self, symbol: &str) -> String {
        let def = self.types.iter().find(|d| d.symbol == symbol);
        self.modules.describe(def.map_or(ROOT, |d| d.module))
    }

    pub(crate) fn adts(&self) -> &Adts {
        &self.adts
    }

    /// What the borrow checker needs to know about the bodies checked.
    pub(crate) fn typed(&self) -> &Typed {
        &self.typed
    }

    /// `t` with the type variables in it replaced by what they turned out
    /// to be, and the lifetimes of closures' results too.
    pub(crate) fn solution(&self, t: &Type) -> Type {
        self.closure_regions(&self.decided_regions(&self.solved(t)))
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
            span: l.def(),
            ty: l.ty.clone(),
            mutable: l.mutable,
            mut_span: l.mut_span,
            by_ref: l.by_ref,
        };
        self.typed.binds.insert(l.id(), bind);
        for site in &l.sites {
            self.typed.sites.insert(site.node, l.id());
        }
    }

    /// What the name of type `t` refers to, if it's a struct or enum, an
    /// outer type parameter (see `names.rs`) or an opaque type.
    fn type_name(&self, t: &TypeExpr) -> Option<TypeName> {
        if let Some(symbol) = self.opaque_at.get(&t.id) {
            let d = &self.traits.opaques[symbol];
            let (lifetimes, params) = (d.lifetimes.clone(), d.params.clone());
            return Some(TypeName::Opaque(symbol.clone(), lifetimes, params));
        }
        match self.type_names.get(&t.id)? {
            Scoped::Adt(symbol) if let Some(def) = self.aliases.get(symbol) => {
                Some(TypeName::Alias(def.clone()))
            }
            Scoped::Adt(symbol) => {
                let (lts, lengths) = self.arities.get(symbol)?.clone();
                Some(TypeName::Adt(symbol.clone(), lts, lengths))
            }
            Scoped::OuterParam(keyword, item) => Some(TypeName::OuterParam(keyword, item.clone())),
        }
    }

    /// The symbol of the struct or enum that the type name written at
    /// `span` refers to, if it's one.
    fn adt_symbol(&self, id: NodeId) -> Option<&str> {
        match self.type_names.get(&id)? {
            Scoped::Adt(symbol) => Some(symbol),
            Scoped::OuterParam(..) => None,
        }
    }

    /// A type written where type parameters `params` are in scope, and
    /// lifetimes mean what `lts` says.
    fn resolve_with(
        &self,
        t: &TypeExpr,
        params: &Params,
        lts: &mut Lifetimes,
    ) -> Result<Type, Error> {
        resolve(
            t,
            &|_, t| self.type_name(t),
            &|s, tr, name, t, k| self.projection(s, tr, name, t, k),
            params,
            lts,
        )
    }

    /// Effects written where type and effect parameters `params` are in
    /// scope, and lifetimes mean what `lts` says.
    fn resolve_effects_with(
        &self,
        e: &EffectSet,
        params: &Params,
        lts: &mut Lifetimes,
    ) -> Result<Type, Error> {
        resolve_effects(
            e,
            &|_, t| self.type_name(t),
            &|s, tr, name, t, k| self.projection(s, tr, name, t, k),
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

    /// Rejects polymorphic recursion: see `generics::polymorphic_recursion`.
    /// The graph is of everything monomorphisation copies: functions, and
    /// the derived `Eq`s' `eq`s; structs and enums, and impls' associated
    /// types (as `assoc_regular`'s); and each impl's dictionaries, from
    /// which its methods and associated types are reached, with an edge to
    /// one from wherever a proof picks the impl (see `traits/dicts.rs`).
    ///
    /// Invariant: every copy monomorphisation can make is of a use here.
    /// So every call or naming of a generic function a body can make, at
    /// its type arguments, is in `info.instances`: calls, method calls,
    /// paths, and `==`, which calls `<Eq>::eq`; and the impl a trait's
    /// method runs is picked by a proof that's here, at a call on a type
    /// with a head, or where a bound was proved by the impl. So is every
    /// call made by something that isn't a function of the program: a
    /// derived `Eq`'s `==` on each field, below, and a `discard` of its
    /// own, made with each copy of its type (`discard_uses`). And every
    /// type a copy mentions is: in its signature, its instances' type
    /// arguments, and its hidden types. Whatever makes monomorphisation
    /// copy something new must add its uses here too.
    ///
    /// A projection in a type argument, `f::<T::Out>`, can stand for a
    /// type as much bigger than `T` as an impl likes, so an edge through it
    /// has unbounded weight (`generics::polymorphic_recursion`), and any
    /// cycle through it is rejected.
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
        // own.
        let derived_eq: Vec<Option<String>> = self
            .traits
            .impls
            .iter()
            .map(|d| {
                let eq = d.derived && d.trait_name.as_deref() == Some(EQ);
                eq.then(|| format!("<{} as Eq>::eq", d.self_ty.symbols()))
            })
            .collect();
        let (dicts, type_dicts) = traits::dict_names(&self.traits.impls);
        let ix = self.index((&dicts, &type_dicts), &derived_eq);
        let nodes = self.assoc_nodes();
        let eq = ix.method_symbol(&traits::trait_method_symbol(EQ, "eq"));
        let mut uses = Vec::new();
        for (k, d) in self.traits.impls.iter().enumerate() {
            let (Some(from), Type::Adt(symbol, ..)) = (&derived_eq[k], &d.self_ty) else {
                continue;
            };
            for (_, f) in traits::fields(&self.adts[symbol].kind) {
                self.call_uses(&ix, (from, d.span), eq, &[f], &mut uses);
            }
        }
        // Which function each use is in, from where they are.
        for (id, instance) in &self.info.instances {
            let span = &self.instance_at[id];
            // The innermost function it's in.
            let from = self
                .fns
                .iter()
                .filter(|f| f.body.start <= span.start && span.end <= f.body.end)
                .min_by_key(|f| f.body.end - f.body.start);
            let Some(from) = from else { continue };
            let mut site = traits::Site::types(&ix, &from.symbol, *span, &mut uses);
            match instance {
                Instance::Fn(_, args) => {
                    for t in args {
                        self.mention_uses(&mut site, t);
                    }
                }
                Instance::Adt(name, args) => {
                    let t = Type::Adt(name.clone(), Vec::new(), args.clone());
                    self.mention_uses(&mut site, &t);
                }
            }
            if let Instance::Fn(to, args) = instance {
                self.call_uses(&ix, (&from.symbol, *span), to, args, &mut uses);
            }
        }
        // Each function's copies mention the types in its signature.
        for f in &self.fns {
            let mut site = traits::Site::types(&ix, &f.symbol, f.name.span, &mut uses);
            self.mention_uses(&mut site, &f.scheme.ty);
        }
        // What an opaque type's hidden type implements was proved in its
        // function, which picked the impls; and its copies mention it.
        for (symbol, o) in &self.traits.opaques {
            let (Some(hidden), Some(from)) = (&o.hidden, ix.fn_symbol(&o.owner)) else {
                continue;
            };
            let generic = o.generic(symbol);
            let mut site = traits::Site::new(&ix, from, o.span, &mut uses);
            for p in &o.preds {
                let t = replace(&p.ty, &generic, hidden);
                self.proof_uses(&mut site, &t, &p.trait_name);
            }
            self.mention_uses(
                &mut traits::Site::types(&ix, from, o.span, &mut uses),
                hidden,
            );
        }
        uses.extend(self.dict_uses(&ix, &nodes, false));
        uses.extend(self.unknown_uses(&ix));
        uses.extend(self.type_graph(&ix, &nodes));
        uses.extend(self.discard_uses());
        // In source order, so the first one is reported.
        uses.sort_by_key(|u| u.span.start);
        let params = self.graph_params(&ix, &nodes);
        generics::polymorphic_recursion(&uses, &params, &|s| self.current(s))
    }

    /// Is `span` in the package being checked, rather than one before it?
    /// What breaks a rule only with the packages before it is its fault,
    /// as each of those kept the rules on its own.
    pub(super) fn current(&self, span: Span) -> bool {
        span.start >= self.spans_from
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
}

/// `t` with each `from` in it replaced by `to`.
fn replace(t: &Type, from: &Type, to: &Type) -> Type {
    match t == from {
        true => to.clone(),
        false => t.map(|p| replace(p, from, to)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    impl Checker {
        /// What checking the libraries left, as `base` has it, which a
        /// program's checking must leave as it is: their traits, impls,
        /// opaque types, closures, structs and enums, functions' types and
        /// modules' names. Shown in a fixed order, as hash maps have none.
        fn library_part(&self, base: &Checker) -> Vec<String> {
            let mut out = Vec::new();
            let sorted = |keys: Vec<&String>| {
                let mut keys: Vec<String> = keys.into_iter().cloned().collect();
                keys.sort();
                keys
            };
            for name in sorted(base.traits.traits.keys().collect()) {
                out.push(format!("trait {name}: {:?}", self.traits.traits[&name]));
            }
            for d in &self.traits.impls[..base.traits.impls.len()] {
                out.push(format!("impl {d:?}"));
            }
            for s in sorted(base.traits.opaques.keys().collect()) {
                out.push(format!("opaque {s}: {:?}", self.traits.opaques[&s]));
            }
            for s in sorted(base.traits.closures.keys().collect()) {
                out.push(format!("closure {s}: {:?}", self.traits.closures[&s]));
            }
            for s in sorted(base.adts.keys().collect()) {
                out.push(format!("type {s}: {:?}", self.adts[&s]));
            }
            for f in &self.fns[..base.fns.len()] {
                out.push(format!("fn {}: {:?}", f.symbol, f.scheme));
            }
            // Not the crate root's, which is the program's.
            for m in &self.modules.mods[1..base.modules.mods.len()] {
                for (ns, table) in [("type", &m.types), ("value", &m.values)] {
                    for name in sorted(table.keys().collect()) {
                        let item = table[&name].item;
                        out.push(format!("{} {ns} {name}: {item:?}", m.path));
                    }
                }
            }
            out
        }
    }

    /// Each library is checked on its own, before any program, against
    /// those before it: core alone, then alloc. Checking a program goes on
    /// from there and must leave what's known of them as it was, whether
    /// the program checks or not: else a library would mean something
    /// different with some programs than alone.
    #[test]
    fn checking_a_program_leaves_the_libraries_as_they_were() {
        let libraries = libraries();
        let base = libraries.cx.lock().unwrap().clone();
        let want = base.library_part(&base);
        assert!(want.len() > 100, "only {} things known", want.len());
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "t"))
            .collect();
        paths.sort();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let failures: Vec<String> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    s.spawn(|| {
                        let base = libraries.cx.lock().unwrap().clone();
                        let mut failures = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(path) = paths.get(i) else {
                                return failures;
                            };
                            let src = std::fs::read_to_string(path).unwrap();
                            let (_, prog) = crate::load::load(path, &src, &crate::no_files);
                            let Ok(mut prog) = prog else {
                                continue;
                            };
                            let mut cx = base.clone();
                            let _ = cx.package(&mut prog, ROOT, libraries.prog.counts());
                            let got = cx.library_part(&base);
                            if let Some((w, g)) = want.iter().zip(&got).find(|(w, g)| w != g) {
                                let name = path.file_name().unwrap().to_string_lossy();
                                failures.push(format!("{name}:\n  was {w}\n  now {g}"));
                            }
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
