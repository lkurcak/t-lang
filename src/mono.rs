//! Monomorphisation: turns a checked program with generics into one
//! without, by making a copy of each generic function, struct and enum for
//! each list of type arguments it's used with. `id(1)` and `id(true)` call
//! two different functions, `id::<i64>` and `id::<bool>`, each of which
//! is plain non-generic code. See docs/04-generics.md.
//!
//! The copies are named after what they're copies of, like `id::<i64>` and
//! `Option::<i64>`. No name in the source can look like that, so they can't
//! clash with anything.
//!
//! The checker has already worked out the type arguments of every use of
//! something generic (`Info::instances`). In a generic function those can
//! mention its own type parameters; the copy for `T = i64` substitutes
//! `i64` for them, and so on down. It finishes because the checker rejects
//! polymorphic recursion.
//!
//! It works on a program with no nested functions or local types left (see
//! `lift.rs`), so everything has a name of its own.
//!
//! A call of a trait's method, like `<Show>::show` at `Self = T`, becomes a
//! call of the impl's: once `T` is known, as it is in each copy, trait
//! resolution finds the one impl of `Show` for it (see `traits.rs`), and the
//! copy is of that impl's `show`, or the trait's default if it has none.
//! It's named after the type and trait, as `<Point as Show>::show`, and an
//! inherent method's after the type, as `<Point>::len`. Traits and impls
//! are gone after this: all that's left of them is plain functions, and
//! which types are `Copy` and `Eq`, which each type's copy says with a
//! `#[derive(..)]`. What the program doesn't spell out, dropping a value
//! and `==` of a built-in or derived `Eq`, becomes *glue*: MIR functions
//! written here, per type (`glue.rs`). A `drop` in a body is of its type's
//! drop glue, left out where that does nothing; an `==` of what isn't a
//! number, `bool`, character or `()` calls the type's own `eq` or its glue,
//! as `<` calls `Ord`'s method.
//!
//! An opaque type, `impl Fn(i64) -> i64` in a function's return type, is
//! replaced by the type it stands for, in the copy at the type arguments
//! it's at (`Projections::reveal`). That's sound for the same reason as for
//! a trait's method: the checker only let callers rely on its bounds, which
//! that type implements, by the impls it'll find here.
//!
//! What a function does is its MIR (`mir.rs`), which the borrow checker
//! built and checked at the function's own types, and kept: each copy is a
//! copy of that, with the copy's types, and calls of what's generic naming
//! the copies they call (`Cx::body`). A closure's MIR is copied with the
//! function it's in. Nothing about control flow changes, so what the
//! borrow checker proved of the generic body holds of every copy. The
//! drops the body's drop elaboration worked out (`borrowck/drops.rs`) are
//! copied too: they depend only on what's moved, which is the same in
//! every copy.
//!
//! The interpreter runs what comes out of here, and `t mono` prints it.

use crate::ast::*;
use crate::check::{self, Info, Instance};
use crate::error::Span;
use crate::traits::{COPY, DISCARD, EQ, FnKind};
use crate::traits::{Projections, Source};
use crate::types::{Params, Region, Subst, Type};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod glue;
pub(crate) use glue::{glue_name, is_glue};

/// A lifted, monomorphised program, ready for the interpreter.
pub struct Executable {
    program: Program,
    /// The MIR of each function's copy that has a body, and each
    /// closure's, by name (see `Mono::bodies`).
    bodies: HashMap<String, crate::mir::Body>,
    /// For `main` and each test that gives back a `Result`, by symbol,
    /// the copy of `std::rt::report` that reports its error.
    reports: HashMap<String, String>,
    /// For `main` and each test, by symbol, the drop glue of what it gives
    /// back, if dropping that does anything.
    entry_drops: HashMap<String, String>,
}

impl Executable {
    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn into_program(self) -> Program {
        self.program
    }

    /// The MIR body of copy `name`, of a function or a closure.
    pub(crate) fn body(&self, name: &str) -> Option<&crate::mir::Body> {
        self.bodies.get(name)
    }

    /// The same, with its name, as long as the body lives.
    pub(crate) fn body_named(&self, name: &str) -> Option<(&str, &crate::mir::Body)> {
        let (name, b) = self.bodies.get_key_value(name)?;
        Some((name.as_str(), b))
    }

    /// The copy of `std::rt::report` for the error that `entry`, `main`
    /// or a test, gives back, if it gives back a `Result`.
    pub fn report(&self, entry: &str) -> Option<&str> {
        self.reports.get(entry).map(String::as_str)
    }

    /// The drop glue of what `entry`, `main` or a test, gives back, if
    /// dropping that does anything.
    pub(crate) fn entry_drop(&self, entry: &str) -> Option<&str> {
        self.entry_drops.get(entry).map(String::as_str)
    }
}

/// What `t mono` prints: the structs and enums made, then each function,
/// its MIR for a body, and after it the closures made in it.
impl std::fmt::Display for Executable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render_only(&|_| true))
    }
}

impl Executable {
    /// What `t mono` prints of the structs, enums and functions whose names
    /// `keep` says.
    pub fn render_only(&self, keep: &dyn Fn(&str) -> bool) -> String {
        use std::fmt::Write;
        let mut f = String::new();
        let types = Program {
            fns: Vec::new(),
            types: (self.program.types.iter())
                .filter(|d| keep(&d.name().name))
                .cloned()
                .collect(),
            ..self.program.clone()
        };
        let types = types.to_string();
        f.push_str(&types);
        let mut first = types.trim().is_empty();
        let f = &mut f;
        for d in self.program.fns.iter().filter(|d| keep(&d.name.name)) {
            if !std::mem::take(&mut first) {
                f.push('\n');
            }
            let name = &d.name.name;
            let Some(body) = self.bodies.get(name) else {
                let _ = writeln!(f, "{}", crate::pretty::fn_text(d).trim_end());
                continue;
            };
            let head = crate::pretty::fn_head(d);
            let _ = write!(f, "{head}{{\n{}}}\n", body.render("    ", self));
            let mut closures: Vec<(&String, &crate::mir::Body)> = (self.bodies.iter())
                .filter(|(c, _)| c.starts_with(&format!("{name}::{{closure#")))
                .collect();
            closures.sort_by_key(|(c, _)| c.as_str());
            for (c, b) in closures {
                let _ = write!(f, "\nclosure {c} {{\n{}}}\n", b.render("    ", self));
            }
        }
        // Then the glue, by name, of the types kept.
        let mut glue: Vec<(&String, &crate::mir::Body)> = (self.bodies.iter())
            .filter(|(g, b)| is_glue(g) && b.locals.get(1).is_some_and(|l| keeps(&l.ty, keep)))
            .collect();
        glue.sort_by_key(|(g, _)| g.as_str());
        for (g, b) in glue {
            if !std::mem::take(&mut first) {
                f.push('\n');
            }
            let _ = write!(f, "glue {g} {{\n{}}}\n", b.render("    ", self));
        }
        std::mem::take(f)
    }
}

/// Does `keep` say to show the glue of a value of type `t`: of a struct or
/// enum it keeps, through references, boxes, arrays and slices, or of a
/// closure?
fn keeps(t: &Type, keep: &dyn Fn(&str) -> bool) -> bool {
    match t {
        Type::Adt(name, _, _) if crate::types::is_closure(name) => true,
        Type::Adt(name, _, args) => keep(&mangle(name, args)),
        Type::Ref(_, _, inner) | Type::Box(inner) | Type::Slice(inner) | Type::Array(inner, _) => {
            keeps(inner, keep)
        }
        _ => true,
    }
}

impl crate::mir::Names for Executable {
    fn field(&self, ty: &str, i: usize) -> Option<String> {
        self.program.types.iter().find_map(|d| match d {
            TypeDecl::Struct(s) if s.name.name == ty => Some(s.fields.get(i)?.name.name.clone()),
            _ => None,
        })
    }

    fn variant(&self, ty: &str, i: usize) -> Option<String> {
        self.program.types.iter().find_map(|d| match d {
            TypeDecl::Enum(e) if e.name.name == ty => Some(e.variants.get(i)?.name.name.clone()),
            _ => None,
        })
    }
}

/// `prog`, which has been checked and lifted, without generics.
///
/// Nothing a copy does can be wrong where the generic function wasn't: the
/// checker checked it for every type its type parameters could be, and
/// what a copy does depends on its types only through the impls the bounds
/// said there were.
pub fn monomorphise(prog: crate::lift::Lifted<'_>) -> Executable {
    let mut mono = Mono::new(&prog.program, prog.info);
    let program = mono.run();
    Executable {
        program,
        bodies: mono.bodies,
        reports: mono.reports,
        entry_drops: mono.entry_drops,
    }
}

/// The name of the copy of the closure made by the lambda with node `id`,
/// in the copy of a function `root`.
pub(crate) fn closure_copy(root: &str, id: NodeId) -> String {
    format!("{root}::{{closure#{}}}", id.0)
}

/// The std library's function that reports the error `main` gives back.
const REPORT: &str = "std::rt::report";

/// The name of the copy of `name` for type arguments `args`. It has the
/// symbols of the types in them, which tell apart types of the same name,
/// where `Type`'s `Display` shows the names as written.
pub(crate) fn mangle(name: &str, args: &[Type]) -> String {
    match args {
        [] => name.to_string(),
        args => {
            let args: Vec<String> = args.iter().map(Type::symbols).collect();
            format!("{name}::<{}>", args.join(", "))
        }
    }
}

/// A copy of a generic function: what it's a copy of, at which type
/// arguments.
struct Copy {
    of: String,
    args: Vec<Type>,
}

/// A function to make copies of: its declaration, all its type parameters
/// (for a method, its impl's or `Self` first), what `Self` is in it, for a
/// method of an impl, and its impl's lifetime parameters.
struct Decl<'p> {
    f: &'p FnDecl,
    params: Params,
    self_ty: Option<Type>,
    lifetimes: Vec<Ident>,
}

/// What a call of a method names, once it's known what type it's of.
enum Target {
    /// Function `.0` at type arguments `.1`, whose copy is called `.2`.
    Fn(String, Vec<Type>, String),
    /// The `eq` of a built-in or derived `Eq`, of the type: `==`.
    Eq(Type),
    /// The `discard` of a built-in or derived `Discard`, of the type:
    /// dropping it, which the interpreter does part by part.
    Discard(Type),
}

struct Mono<'p> {
    prog: &'p Program,
    info: &'p Info,
    arities: HashMap<String, (usize, Vec<bool>)>,
    /// What each type alias stands for, by symbol: a use of one is a use
    /// of that.
    aliases: HashMap<String, std::sync::Arc<check::AliasDef>>,
    fns: HashMap<&'p str, Decl<'p>>,
    types: HashMap<&'p str, &'p TypeDecl>,
    /// Every copy of a generic function so far, by name, and their names in
    /// the order they were first needed.
    copies: HashMap<String, Copy>,
    copy_order: Vec<String>,
    /// Copies of functions and types still to make, in the order they were
    /// first needed.
    fn_queue: Vec<String>,
    type_queue: Vec<(String, Vec<Type>)>,
    /// The copies of each generic type, in that order.
    type_copies: HashMap<String, Vec<Vec<Type>>>,
    /// Whether dropping a value of each type does anything, as worked out
    /// so far (`Cx::needs_drop`).
    needs_drop: HashMap<Type, bool>,
    /// The glue made so far, by name (see `glue.rs`).
    glue_made: HashSet<String>,
    /// The drop glue of what each entry gives back (`Executable::entry_drop`).
    entry_drops: HashMap<String, String>,
    /// What projections stand for, by the impls: in a copy, every type is
    /// known, so every one does.
    proj: Projections,
    /// `Program::next_id`: the nodes made here that weren't in the program
    /// take ids from it. A copy of one that was keeps its id, so the copies
    /// of a generic function share theirs.
    next_id: u32,
    /// The report of each entry's error, by entry (`Executable::report`).
    reports: HashMap<String, String>,
    /// The MIR body of each copy of a function, by its name, and of each
    /// closure, named after the copy of the function it's in, as
    /// `{copy}::{closure#7}`, by its lambda's node: what it does depends on
    /// that copy's type arguments.
    bodies: HashMap<String, crate::mir::Body>,
}

impl<'p> Mono<'p> {
    /// An id for a node made here (see `NodeId`).
    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    fn new(prog: &'p Program, info: &'p Info) -> Mono<'p> {
        Mono {
            prog,
            info,
            arities: prog
                .types
                .iter()
                .filter(|d| !matches!(d, TypeDecl::Alias(_)))
                .map(|d| {
                    let arity = (d.generics().lifetimes.len(), d.generics().lengths());
                    (d.name().name.clone(), arity)
                })
                .collect(),
            aliases: prog
                .types
                .iter()
                .filter_map(|d| match d {
                    TypeDecl::Alias(a) => {
                        Some((a.name.name.clone(), Arc::new(check::AliasDef::new(a))))
                    }
                    _ => None,
                })
                .collect(),
            fns: decls(prog, info),
            types: prog
                .types
                .iter()
                .map(|d| (d.name().name.as_str(), d))
                .collect(),
            copies: HashMap::new(),
            copy_order: Vec::new(),
            fn_queue: Vec::new(),
            type_queue: Vec::new(),
            type_copies: HashMap::new(),
            needs_drop: HashMap::new(),
            glue_made: HashSet::new(),
            entry_drops: HashMap::new(),
            proj: Projections::new(&info.traits),
            next_id: prog.next_id,
            reports: HashMap::new(),
            bodies: HashMap::new(),
        }
    }

    fn run(&mut self) -> Program {
        let prog = self.prog;
        // Non-generic functions and types are where it starts: each is
        // its own one copy, and from them come all the others.
        let mut fns: HashMap<String, FnDecl> = HashMap::new();
        for f in prog.fns.iter().filter(|f| f.generics.types.is_empty()) {
            fns.insert(
                f.name.name.clone(),
                self.function(f, &Subst::default(), &Params::default(), None),
            );
        }
        // The report of `main`'s error, and each test's, which the
        // interpreter calls.
        let entries = (self.info.main_error.clone())
            .map(|e| ("main".to_string(), e))
            .into_iter()
            .chain(
                (self.info.tests.iter()).filter_map(|t| Some((t.symbol.clone(), t.error.clone()?))),
            );
        let entries: Vec<(String, Type)> = entries.collect();
        let mut roots = Vec::new();
        for (entry, e) in entries {
            let report = self.named_copy(REPORT, vec![e.clone()], mangle(REPORT, &[e]));
            self.reports.insert(entry, report.clone());
            roots.push(report);
        }
        let mut types: HashMap<String, TypeDecl> = HashMap::new();
        let adts = prog
            .types
            .iter()
            .filter(|d| !matches!(d, TypeDecl::Alias(_)));
        for d in adts.filter(|d| d.type_params().is_empty()) {
            types.insert(d.name().name.clone(), self.type_decl(d, &[]));
        }
        loop {
            if let Some(name) = self.fn_queue.pop() {
                let copy = &self.copies[&name];
                let d = &self.fns[copy.of.as_str()];
                let (f, params) = (d.f, d.params.clone());
                let lifetimes = d.lifetimes.clone();
                let mut s = Subst::new(&params, &copy.args.clone());
                if let Some(t) = &d.self_ty {
                    let t = s.apply(t);
                    s = s.with_type("Self", t);
                }
                let mut decl = self.function(f, &s, &params, Some(&name));
                decl.generics.lifetimes.splice(0..0, lifetimes);
                fns.insert(name, decl);
            } else if let Some((name, args)) = self.type_queue.pop() {
                let d = self.types[name.as_str()];
                types.insert(mangle(&name, &args), self.type_decl(d, &args));
            } else {
                break;
            }
        }

        // In source order, with each generic one's copies where it was.
        // Doc comments that document nothing don't matter to the program.
        let mut out = Program {
            mods: prog.mods.clone(),
            core: None,
            libraries: Vec::new(),
            packages: Vec::new(),
            uses: Vec::new(),
            types: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            fns: Vec::new(),
            stray_docs: Vec::new(),
            next_id: self.next_id,
            proved: prog.proved.clone(),
            measures: prog.measures.clone(),
        };
        for d in &prog.types {
            let name = &d.name().name;
            if d.type_params().is_empty() {
                out.types.extend(types.remove(name));
            } else {
                for args in self.type_copies.get(name).into_iter().flatten() {
                    out.types.extend(types.remove(&mangle(name, args)));
                }
            }
        }
        for f in &prog.fns {
            let name = &f.name.name;
            if f.generics.types.is_empty() {
                out.fns.extend(fns.remove(name));
            } else {
                for copy in &self.copy_order {
                    if self.copies[copy].of == *name {
                        out.fns.extend(fns.remove(copy));
                    }
                }
            }
        }
        // Then the methods, in the order they were first needed.
        for copy in &self.copy_order {
            out.fns.extend(fns.remove(copy));
        }
        self.without_unused_core(&mut out, roots);
        out
    }

    /// `out` without the core library's functions, structs and enums that
    /// are made whether they're used or not, as they aren't generic, and
    /// aren't: they'd only be noise in what `t mono` prints. A copy of
    /// anything generic is only made where it's used.
    /// Those named in `roots` are used, whether anything calls them or not.
    fn without_unused_core(&self, out: &mut Program, roots: Vec<String>) {
        // Copies of the core library's generic types too, which its
        // functions' signatures may make, like `Option<i64>`.
        let lazy = |name: &str| {
            let of = name.split("::<").next().unwrap_or(name);
            // Only the built-in libraries give slices, strings and
            // characters methods, or implement traits for them.
            let builtin = ["<[", "<str", "<char"].iter().any(|p| name.starts_with(p));
            // An inherent method of a library's type, `<alloc::string::String>::new`:
            // not a trait's, whose `eq` `==` may call without naming it.
            let inherent = (name.strip_prefix('<'))
                .and_then(|rest| rest.split_once(">::"))
                .filter(|(ty, _)| !ty.contains(" as "))
                .map(|(ty, _)| ty.split("::<").next().unwrap_or(ty));
            builtin
                || self.info.library_symbols.contains(name)
                || self.info.library_symbols.contains(of)
                || inherent.is_some_and(|ty| self.info.library_symbols.contains(ty))
        };
        let mut used: HashSet<String> = roots.into_iter().collect();
        // What glue calls is used: nothing names glue, but a drop or `==`
        // may run it.
        for (name, body) in &self.bodies {
            if is_glue(name) {
                used::in_body(body, &mut used);
            }
        }
        let mut kept_types: Vec<bool> = out.types.iter().map(|d| !lazy(&d.name().name)).collect();
        let mut kept_fns: Vec<bool> = out.fns.iter().map(|f| !lazy(&f.name.name)).collect();
        let mut seen_types = vec![false; out.types.len()];
        let mut seen_fns = vec![false; out.fns.len()];
        loop {
            let mut changed = false;
            for (i, d) in out.types.iter().enumerate() {
                kept_types[i] |= used.contains(&d.name().name);
                if kept_types[i] && !seen_types[i] {
                    seen_types[i] = true;
                    changed = true;
                    used::in_type_decl(d, &mut used);
                    used.insert(format!("<{} as {DISCARD}>::discard", d.name().name));
                }
            }
            for (i, f) in out.fns.iter().enumerate() {
                kept_fns[i] |= used.contains(&f.name.name);
                if kept_fns[i] && !seen_fns[i] {
                    seen_fns[i] = true;
                    changed = true;
                    used::in_fn(f, &mut used);
                    let name = &f.name.name;
                    let closures = format!("{name}::{{closure#");
                    for (b, body) in &self.bodies {
                        if b == name || b.starts_with(&closures) {
                            used::in_body(body, &mut used);
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut keep = kept_types.into_iter();
        out.types.retain(|_| keep.next().unwrap_or(true));
        let mut keep = kept_fns.into_iter();
        out.fns.retain(|_| keep.next().unwrap_or(true));
    }

    /// The name of the copy of function `name` for `args`, `mangled`,
    /// making it if it's new.
    fn named_copy(&mut self, name: &str, args: Vec<Type>, mangled: String) -> String {
        if !self.copies.contains_key(&mangled) {
            self.fn_queue.push(mangled.clone());
            let copy = Copy {
                of: name.to_string(),
                args,
            };
            self.copies.insert(mangled.clone(), copy);
            self.copy_order.push(mangled.clone());
        }
        mangled
    }

    /// The name of the copy of struct or enum `name` for `args`, making it if
    /// it's new.
    fn type_copy(&mut self, name: &str, args: Vec<Type>) -> String {
        let mangled = mangle(name, &args);
        if !args.is_empty() {
            let copies = self.type_copies.entry(name.to_string()).or_default();
            if !copies.contains(&args) {
                copies.push(args.clone());
                self.type_queue.push((name.to_string(), args));
            }
        }
        mangled
    }

    /// The copy of `f`, which has type parameters `params`, with them
    /// replaced as `s` says, named `name` (or as it was, if it isn't
    /// generic).
    fn function(&mut self, f: &FnDecl, s: &Subst, params: &Params, name: Option<&str>) -> FnDecl {
        let mut params = params.clone();
        params.push_type("Self".into());
        let mut cx = Cx { m: self, s, params };
        let copy = name.unwrap_or(&f.name.name).to_string();
        if name.is_none() && (f.name.name == "main" || f.test.is_some()) {
            let ret = cx.resolved(&f.ret);
            if cx.needs_drop(&ret) {
                let g = cx.drop_glue(&ret);
                cx.m.entry_drops.insert(copy.clone(), g);
            }
        }
        if f.intrinsic.is_none()
            && let Some(b) = cx.m.info.mir.get(&crate::mir::Key::Fn(f.name.name.clone()))
        {
            let b = cx.body(b, &copy);
            cx.m.bodies.insert(copy.clone(), b);
        }
        FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            module: ROOT,
            vis: f.vis,
            name: Ident {
                name: name.unwrap_or(&f.name.name).to_string(),
                span: f.name.span,
                id: f.name.id,
            },
            generics: lifetimes_only(&f.generics),
            params: f
                .params
                .iter()
                .map(|p| Param {
                    doc: None,
                    allows: Vec::new(),
                    mutable: p.mutable,
                    vis: Vis::Private,
                    name: p.name.clone(),
                    ty: cx.ty(&p.ty),
                })
                .collect(),
            parens: f.parens,
            ret: cx.ty(&f.ret),
            effects: {
                // What's inferred, if it doesn't say (see `check/infer.rs`).
                let effects = match (&f.effects, cx.m.info.inferred.get(&f.name.name)) {
                    (None, Some(t)) => cx.subst(t),
                    (e, _) => cx.resolved_effects(e.as_ref()),
                };
                cx.written_effects(&effects, f.span)
            },
            // What it does is its MIR (`Mono::bodies`).
            body: Expr {
                kind: ExprKind::Block(Vec::new(), None),
                span: f.body.span,
                id: f.body.id,
            },
            intrinsic: f.intrinsic,
            measure: f.measure,
            test: f.test,
            ensures: f.ensures.iter().map(|e| *cx.pred(e)).collect(),
        }
    }

    /// The copy of struct or enum `d` for type arguments `args`.
    fn type_decl(&mut self, d: &TypeDecl, args: &[Type]) -> TypeDecl {
        let params = names(d.generics());
        let s = Subst::new(&params, args);
        let mut cx = Cx {
            m: self,
            s: &s,
            params,
        };
        let name = Ident {
            name: mangle(&d.name().name, args),
            span: d.name().span,
            id: d.name().id,
        };
        // Whether it's `Discard`, `Copy` and `Eq`, which the checker of what
        // this makes takes the copy's word for.
        let ty = Type::Adt(d.name().name.clone(), Vec::new(), args.to_vec());
        let traits = &cx.m.info.traits;
        let names: Vec<Ident> = [DISCARD, COPY, EQ]
            .into_iter()
            .filter(|tr| traits.implements(&ty, tr, &[], &[]).is_ok())
            .map(|tr| Ident {
                name: tr.rsplit("::").next().expect("a path").into(),
                span: d.name().span,
                id: cx.m.id(),
            })
            .collect();
        // A `discard` of its own is called wherever a value of it is
        // dropped, which nothing names: so it's made with the type.
        if let Ok(Source::Impl(i, _)) = traits.implements(&ty, DISCARD, &[], &[])
            && !traits.impls[i].derived
        {
            let name = format!("<{DISCARD}>::discard");
            if let Target::Fn(of, args, copy) = cx.target(&name, vec![ty.clone()]) {
                cx.m.named_copy(&of, args, copy);
            }
        }
        let derives = match names.is_empty() {
            true => Vec::new(),
            false => vec![Derive {
                span: d.name().span,
                wheres: vec![None; names.len()],
                names,
            }],
        };
        match d {
            TypeDecl::Struct(st) => TypeDecl::Struct(StructDecl {
                doc: st.doc.clone(),
                allows: st.allows.clone(),
                span: st.span,
                module: ROOT,
                vis: st.vis,
                name,
                generics: lifetimes_only(&st.generics),
                derives,
                fields: st
                    .fields
                    .iter()
                    .map(|f| Param {
                        doc: f.doc.clone(),
                        allows: f.allows.clone(),
                        mutable: None,
                        vis: f.vis,
                        name: f.name.clone(),
                        ty: cx.ty(&f.ty),
                    })
                    .collect(),
                intrinsic: st.intrinsic,
            }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl {
                doc: e.doc.clone(),
                allows: e.allows.clone(),
                span: e.span,
                module: ROOT,
                vis: e.vis,
                name,
                generics: lifetimes_only(&e.generics),
                derives,
                variants: e
                    .variants
                    .iter()
                    .map(|v| Variant {
                        doc: v.doc.clone(),
                        allows: v.allows.clone(),
                        span: v.span,
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|t| cx.ty(t)).collect(),
                    })
                    .collect(),
            }),
            TypeDecl::Alias(_) => unreachable!("an alias stands for a type, and isn't one"),
        }
    }
}

/// The type, effect and const parameters `g` declares.
fn names(g: &Generics) -> Params {
    Params::new(
        g.types.iter().map(|p| p.name.clone()).collect(),
        g.effect_names(),
    )
    .with_consts(g.const_names())
}

/// Every function of `prog`, which checked with `info` and was lifted, by
/// its name: its symbol. Methods too, those of impls and traits' defaults.
fn decls<'p>(prog: &'p Program, info: &Info) -> HashMap<&'p str, Decl<'p>> {
    let mut out: HashMap<&str, Decl> = prog
        .fns
        .iter()
        .map(|f| {
            let d = Decl {
                f,
                params: names(&f.generics),
                self_ty: None,
                lifetimes: Vec::new(),
            };
            (f.name.name.as_str(), d)
        })
        .collect();
    for d in &prog.impls {
        for f in &d.methods {
            let symbol = f.name.name.as_str();
            let def = info
                .traits
                .impls
                .iter()
                .find(|i| i.methods.iter().any(|(_, s)| s == symbol))
                .expect("a method of an impl");
            let params = def.params.concat(&names(&f.generics));
            let decl = Decl {
                f,
                params,
                self_ty: Some(def.self_ty.clone()),
                lifetimes: d.generics.lifetimes.clone(),
            };
            out.insert(symbol, decl);
        }
    }
    for t in &prog.traits {
        for m in t.methods.iter().filter(|m| m.provided) {
            let params = Params::types(vec!["Self".to_string()]).concat(&names(&m.decl.generics));
            let decl = Decl {
                f: &m.decl,
                params,
                self_ty: None,
                lifetimes: Vec::new(),
            };
            out.insert(m.decl.name.name.as_str(), decl);
        }
    }
    out
}

/// `g` without its type parameters, which a copy doesn't have, and the
/// bounds on them. Copies are generic over lifetimes as before.
fn lifetimes_only(g: &Generics) -> Generics {
    Generics {
        lifetimes: g.lifetimes.clone(),
        types: Vec::new(),
        effects: Vec::new(),
        consts: Vec::new(),
        bounds: g
            .bounds
            .iter()
            .filter(|(p, _)| p.name.starts_with('\''))
            .cloned()
            .collect(),
        span: g.span,
        wheres: Vec::new(),
        where_span: None,
    }
}

/// Copying one function or type.
struct Cx<'m, 'p, 's> {
    m: &'m mut Mono<'p>,
    /// What each type parameter of the one being copied stands for.
    s: &'s Subst,
    /// Its type parameters.
    params: Params,
}

impl Cx<'_, '_, '_> {
    /// A type written in the original, as written in the copy.
    fn ty(&mut self, t: &TypeExpr) -> TypeExpr {
        // A refinement is kept, for the interpreter to check.
        if let TypeExprKind::Refined(v, base, pred) = &t.kind {
            return TypeExpr {
                kind: TypeExprKind::Refined(v.clone(), Box::new(self.ty(base)), self.pred(pred)),
                span: t.span,
                id: t.id,
            };
        }
        let resolved = self.resolved(t);
        TypeExpr {
            id: t.id,
            ..self.written(&resolved, t.span)
        }
    }

    /// Predicate `e` of a refinement, as it is in the copy: with the length
    /// each const parameter it names is.
    fn pred(&self, e: &Expr) -> Box<Expr> {
        fn go(cx: &Cx, e: &mut Expr) {
            if let Some(name) = cx.m.info.consts.get(&e.id) {
                if let Type::Nat(n) = cx.subst(&Type::Param(name.clone())) {
                    e.kind = ExprKind::Int(n, Some(crate::types::IntTy::Usize));
                }
                return;
            }
            for c in check::modules::children_mut(e) {
                go(cx, c);
            }
        }
        let mut e = e.clone();
        go(self, &mut e);
        Box::new(e)
    }

    /// A type written in the original, as it is in the copy.
    fn resolved(&self, t: &TypeExpr) -> Type {
        self.resolving(|names, proj, params| {
            check::resolve(t, names, proj, params, &mut check::Lifetimes::Any)
        })
    }

    /// Effects written in the original, as they are in the copy: `panic`
    /// and `alloc` only.
    fn resolved_effects(&self, e: Option<&EffectSet>) -> Type {
        match e {
            Some(e) => self.resolving(|names, proj, params| {
                check::resolve_effects(e, names, proj, params, &mut check::Lifetimes::Any)
            }),
            None => Type::no_effects(),
        }
    }

    /// What `go` resolves, given what the names in the original mean, as it
    /// is in the copy.
    fn resolving(
        &self,
        go: impl FnOnce(
            &dyn Fn(&str, &TypeExpr) -> Option<check::TypeName>,
            &check::ProjNames,
            &Params,
        ) -> Result<Type, crate::error::Error>,
    ) -> Type {
        // Everything is at the top level by now, named by its symbol.
        let (arities, aliases) = (&self.m.arities, &self.m.aliases);
        let opaques = &self.m.info.opaques;
        let names = |name: &str, t: &TypeExpr| {
            if let Some(symbol) = opaques.get(&t.id) {
                let d = &self.m.info.traits.opaques[symbol];
                let (lifetimes, params) = (d.lifetimes.clone(), d.params.clone());
                return Some(check::TypeName::Opaque(symbol.clone(), lifetimes, params));
            }
            match self.params.iter().any(|p| p == name) {
                true => None,
                false if let Some(def) = aliases.get(name) => {
                    Some(check::TypeName::Alias(def.clone()))
                }
                false => arities
                    .get(name)
                    .map(|(l, n)| check::TypeName::Adt(name.to_string(), *l, n.clone())),
            }
        };
        let projections = &self.m.info.projections;
        let proj = |s: Type, tr: Option<&Ident>, name: &Ident, t: &TypeExpr, _: check::Kind| {
            let tr = match tr {
                Some(tr) => tr.name.clone(),
                None => projections[&t.id].clone(),
            };
            Ok(Type::Proj(Box::new(s), tr, name.name.clone()))
        };
        let resolved =
            go(&names, &proj, &self.params).expect("monomorphising a program that checked");
        self.subst(&resolved)
    }

    /// `t`, of the original, as it is in the copy: with the copy's type
    /// arguments, every opaque type replaced by the type it stands for,
    /// and every projection worked out, as each can be now.
    fn subst(&self, t: &Type) -> Type {
        self.m.proj.reveal(&self.s.apply(t))
    }

    /// `t`, of the original, as it is in the copy, as `subst` says, and
    /// with the effects inferred for each function whose effects a
    /// function type in it is: a function item's type says them so.
    fn subst_inferred(&self, t: &Type) -> Type {
        self.inferred(&self.subst(t))
    }

    /// `t` with each function's inferred effects in it what they are.
    fn inferred(&self, t: &Type) -> Type {
        match t {
            Type::Inferred(name, args) => {
                let d = &self.m.fns[name.as_str()];
                let effects = &self.m.info.inferred[name];
                self.inferred(&Subst::new(&d.params, args).apply(effects))
            }
            t => t.map(|p| self.inferred(p)),
        }
    }

    /// The type of each field of struct or enum type `t`, which has no
    /// type parameters left in it.
    fn field_types(&mut self, t: &Type) -> Vec<Type> {
        let Type::Adt(name, _, args) = t else {
            unreachable!("fields of `{t}`")
        };
        let d = self.m.types[name.as_str()];
        let params = names(d.generics());
        let s = Subst::new(&params, args);
        let cx = Cx {
            m: &mut *self.m,
            s: &s,
            params,
        };
        let written: Vec<&TypeExpr> = match d {
            TypeDecl::Struct(st) => st.fields.iter().map(|f| &f.ty).collect(),
            TypeDecl::Enum(e) => e.variants.iter().flat_map(|v| &v.fields).collect(),
            TypeDecl::Alias(_) => unreachable!("an alias stands for a type, and isn't one"),
        };
        written
            .into_iter()
            .map(|f| cx.resolved(unrefined(f)))
            .collect()
    }

    /// How to write `t`, which has no type parameters left in it.
    fn written(&mut self, t: &Type, span: Span) -> TypeExpr {
        let kind = match t {
            Type::Int(t) => TypeExprKind::Named(t.name().into(), Vec::new()),
            Type::Bool => TypeExprKind::Named("bool".into(), Vec::new()),
            Type::Char => TypeExprKind::Named("char".into(), Vec::new()),
            Type::Str => TypeExprKind::Named("str".into(), Vec::new()),
            Type::Unit => TypeExprKind::Unit,
            Type::Never => TypeExprKind::Never,
            Type::Fn(sig) => TypeExprKind::Fn(
                sig.params.iter().map(|p| self.written(p, span)).collect(),
                Box::new(self.written(&sig.ret, span)),
                self.written_effects(&sig.effects, span),
            ),
            // A closure's type can't be written; this is only for showing
            // the program.
            Type::Adt(name, _, args) if crate::types::is_closure(name) => {
                let args: Vec<Type> = args.iter().map(Type::erased).collect();
                for a in args.iter().filter(|a| !matches!(a, Type::Effects(_))) {
                    self.written(a, span);
                }
                TypeExprKind::Named(mangle(name, &args), Vec::new())
            }
            Type::Adt(name, regions, args) => {
                // A copy's fields may need more copies.
                for a in args {
                    self.written(a, span);
                }
                // Lifetimes as written, or none if they were all left out.
                let lifetimes = match regions.iter().all(|r| *r == Region::Erased) {
                    true => Vec::new(),
                    false => regions
                        .iter()
                        .map(|r| TypeExpr {
                            kind: TypeExprKind::Lifetime(region_name(r)),
                            span,
                            id: self.m.id(),
                        })
                        .collect(),
                };
                let copy = self
                    .m
                    .type_copy(name, args.iter().map(Type::erased).collect());
                TypeExprKind::Named(copy, lifetimes)
            }
            Type::Ref(r, m, inner) => {
                let lifetime = match r {
                    Region::Erased => None,
                    Region::Named(n) if n.starts_with("'^") => None,
                    r => Some(Ident {
                        name: region_name(r),
                        span,
                        id: self.m.id(),
                    }),
                };
                TypeExprKind::Ref(lifetime, *m, Box::new(self.written(inner, span)))
            }
            Type::Box(inner) => {
                TypeExprKind::Named(crate::check::BOX.into(), vec![self.written(inner, span)])
            }
            Type::Slice(inner) => TypeExprKind::Slice(Box::new(self.written(inner, span))),
            Type::Array(inner, n) => TypeExprKind::Array(
                Box::new(self.written(inner, span)),
                Box::new(self.written(n, span)),
            ),
            Type::Nat(n) => TypeExprKind::Nat(*n),
            Type::Param(_) | Type::Var(_) | Type::Proj(..) | Type::Opaque(..) => {
                unreachable!("`{t}` left after monomorphising")
            }
            Type::Effects(_) | Type::Alloc | Type::Panic | Type::Unsafe | Type::Inferred(..) => {
                unreachable!("effects `{t}` written as a type")
            }
        };
        TypeExpr {
            kind,
            span,
            id: self.m.id(),
        }
    }

    /// How to write effects `t`, which are `alloc`, `panic` and `unsafe` only by now:
    /// nothing, if there are none.
    fn written_effects(&mut self, t: &Type, span: Span) -> Option<EffectSet> {
        let items: Vec<Effect> = t
            .effect_items()
            .iter()
            .map(|e| {
                let name = match e {
                    Type::Alloc | Type::Panic | Type::Unsafe => e.to_string(),
                    // An `Inferred` too: none is left once checking is done.
                    e => unreachable!("effect `{e}` left after monomorphising"),
                };
                let id = self.m.id();
                Effect {
                    kind: EffectKind::Named(Ident { name, span, id }),
                    span,
                    id: self.m.id(),
                }
            })
            .collect();
        if items.is_empty() {
            return None;
        }
        Some(EffectSet {
            braced: items.len() > 1,
            items,
            span,
            with: true,
        })
    }

    /// What function `name`, at type arguments `args` with no type
    /// parameters left in them, is: itself, unless it's a trait's method,
    /// which is the method of the impl for its `Self`. Named for its copy.
    fn target(&mut self, name: &str, args: Vec<Type>) -> Target {
        let traits = &self.m.info.traits;
        let of_trait = traits
            .traits
            .iter()
            .find_map(|(t, d)| Some((t.clone(), d.methods.iter().find(|m| m.symbol == name)?)));
        let impl_of = || {
            let d = traits
                .impls
                .iter()
                .find(|d| d.methods.iter().any(|(_, s)| s == name))?;
            let m = d.methods.iter().find(|(_, s)| s == name)?.0.clone();
            Some((d.clone(), m))
        };
        let (tr, method, self_ty, own) = match (of_trait, impl_of()) {
            (Some((tr, m)), _) => {
                let (self_ty, own) = args.split_first().expect("`Self` first");
                (Some(tr), m.name.clone(), self_ty.clone(), own.to_vec())
            }
            (None, Some((d, m))) => {
                let (of_impl, own) = args.split_at(d.params.len());
                let self_ty = Subst::new(&d.params, of_impl).apply(&d.self_ty);
                // A method of an impl of a trait is only called through
                // the trait's, so this is an inherent one.
                (None, m, self_ty, own.to_vec())
            }
            (None, None) => {
                let copy = mangle(name, &args);
                return Target::Fn(name.to_string(), args, copy);
            }
        };
        let header = match &tr {
            Some(tr) => format!("{} as {tr}", self.copy_name(&self_ty)),
            None => self.copy_name(&self_ty),
        };
        let copy = mangle(&format!("<{header}>::{method}"), &own);
        let Some(tr) = tr else {
            return Target::Fn(name.to_string(), args, copy);
        };
        let traits = &self.m.info.traits;
        match traits.implements(&self_ty, &tr, &[], &[]) {
            Ok(Source::Impl(i, impl_args)) if !traits.impls[i].derived => {
                let d = &traits.impls[i];
                match d.methods.iter().find(|(m, _)| *m == method) {
                    Some((_, symbol)) => {
                        let mut args = impl_args;
                        args.extend(own);
                        Target::Fn(symbol.clone(), args, copy)
                    }
                    // The trait's default.
                    None => Target::Fn(name.to_string(), args, copy),
                }
            }
            Ok(Source::Impl(..) | Source::Builtin) if tr == DISCARD => Target::Discard(self_ty),
            Ok(Source::Impl(..) | Source::Builtin) => Target::Eq(self_ty),
            r => unreachable!("`{self_ty}: {tr}` in a copy: {r:?}"),
        }
    }

    /// How type `t`, with no type parameters left in it, is named in the
    /// name of a copy of a method: with the names of copies of structs and
    /// enums, as values of them are, and no lifetimes.
    fn copy_name(&mut self, t: &Type) -> String {
        match t {
            Type::Adt(name, _, args) => {
                let args: Vec<Type> = args.iter().map(Type::erased).collect();
                self.written(t, Span::new(0, 0));
                mangle(name, &args)
            }
            Type::Proj(..) => unreachable!("`{t}` left after monomorphising"),
            Type::Ref(_, m, inner) => {
                format!("&{}{}", if *m { "mut " } else { "" }, self.copy_name(inner))
            }
            Type::Box(inner) => format!("Box<{}>", self.copy_name(inner)),
            Type::Slice(inner) => format!("[{}]", self.copy_name(inner)),
            Type::Fn(sig) => {
                let params: Vec<String> = sig.params.iter().map(|p| self.copy_name(p)).collect();
                format!("fn({}) -> {}", params.join(", "), self.copy_name(&sig.ret))
            }
            t => t.to_string(),
        }
    }

    /// The copy of MIR body `b`, of the function whose copy is `root` or of
    /// a closure in it: with the copy's types, and calls of what's generic
    /// or of a trait's method naming the copies they call. A closure made
    /// in it gets its copy too.
    fn body(&mut self, b: &crate::mir::Body, root: &str) -> crate::mir::Body {
        use crate::mir::{Agg, Block, Local, LocalDecl, LocalKind, Operand, Place, Rvalue};
        use crate::mir::{Statement, StmtKind};
        let mut out = b.clone();
        for l in &mut out.locals {
            l.ty = self.subst_inferred(&l.ty).erased();
            self.copies_in(&l.ty.clone());
        }
        let mut blocks = Vec::new();
        let temp = |out: &mut crate::mir::Body, ty: Type, span: Span| {
            out.locals.push(LocalDecl {
                ty,
                kind: LocalKind::Temp,
                mutable: true,
                name: None,
                span,
                mut_span: None,
                by_ref: None,
                deferred: false,
            });
            Local(out.locals.len() - 1)
        };
        for block in &b.blocks {
            let mut stmts = Vec::new();
            for s in &block.stmts {
                // A drop that does nothing at the copy's types goes.
                if let StmtKind::Drop(p, t) = &s.kind {
                    let t = self.subst_inferred(t).erased();
                    if self.needs_drop(&t) {
                        self.drop_glue(&t);
                        let kind = StmtKind::Drop(p.clone(), t);
                        stmts.push(Statement { kind, span: s.span });
                    }
                    continue;
                }
                let StmtKind::Assign {
                    place,
                    value,
                    declare,
                } = &s.kind
                else {
                    stmts.push(s.clone());
                    continue;
                };
                // `a == b` of what isn't compared directly calls what
                // compares it, on references, and `a != b` is `!(a == b)`.
                if let Rvalue::Binary(
                    op @ (BinOp::Eq | BinOp::Ne),
                    Operand::Copy(l),
                    Operand::Copy(r),
                    id,
                ) = value
                {
                    let t = self.place_ty(&out.locals, l);
                    if let Some(f) = self.eq_fn(&t) {
                        let r_ty = Type::Ref(Region::Erased, false, Box::new(t));
                        let mut refs = Vec::new();
                        for p in [l, r] {
                            let t = temp(&mut out, r_ty.clone(), s.span);
                            let kind = StmtKind::Assign {
                                place: Place::local(t),
                                value: Rvalue::Ref(0, false, p.clone()),
                                declare: false,
                            };
                            stmts.push(Statement { kind, span: s.span });
                            refs.push(t);
                        }
                        let call = Rvalue::Call(
                            Operand::Const(crate::mir::Const::Fn(f, Vec::new())),
                            refs.iter()
                                .map(|t| Operand::Move(Place::local(*t)))
                                .collect(),
                        );
                        let (dest, not) = match op {
                            BinOp::Eq => (place.clone(), None),
                            _ => {
                                let t = temp(&mut out, Type::Bool, s.span);
                                (Place::local(t), Some(t))
                            }
                        };
                        let kind = StmtKind::Assign {
                            place: dest,
                            value: call,
                            declare: *declare && not.is_none(),
                        };
                        stmts.push(Statement { kind, span: s.span });
                        if let Some(t) = not {
                            let kind = StmtKind::Assign {
                                place: place.clone(),
                                value: Rvalue::Unary(
                                    UnOp::Not,
                                    Operand::Move(Place::local(t)),
                                    *id,
                                ),
                                declare: *declare,
                            };
                            stmts.push(Statement { kind, span: s.span });
                            stmts.push(Statement {
                                kind: StmtKind::Dead(t),
                                span: s.span,
                            });
                        }
                        for t in refs.into_iter().rev() {
                            stmts.push(Statement {
                                kind: StmtKind::Dead(t),
                                span: s.span,
                            });
                        }
                        continue;
                    }
                }
                // A closure called by value whose body takes it by
                // reference: it's put in a temporary, called through a
                // reference to it, and what's left of it dropped after.
                // So is one called directly, as a variable's value.
                let by_value = match value {
                    Rvalue::Call(
                        Operand::Const(crate::mir::Const::Receiver(FnKind::FnOnce)),
                        args,
                    ) => args.first().map(|recv| (recv, &args[1..])),
                    Rvalue::Call(f @ (Operand::Copy(_) | Operand::Move(_)), args) => {
                        Some((f, &args[..]))
                    }
                    _ => None,
                };
                if let Some((recv, rest)) = by_value
                    && let Some(p) = recv.place()
                    && let t = self.place_ty(&out.locals, p)
                    && let Some(kind) = self.by_ref_body(&t)
                {
                    let args = rest;
                    let tmp = temp(&mut out, t.clone(), s.span);
                    let recv = self.operand(recv);
                    let assign = StmtKind::Assign {
                        place: Place::local(tmp),
                        value: Rvalue::Use(recv),
                        declare: false,
                    };
                    stmts.push(Statement {
                        kind: assign,
                        span: s.span,
                    });
                    let m = kind == FnKind::FnMut;
                    let r = temp(
                        &mut out,
                        Type::Ref(Region::Erased, m, Box::new(t.clone())),
                        s.span,
                    );
                    let borrow = StmtKind::Assign {
                        place: Place::local(r),
                        value: Rvalue::Ref(0, m, Place::local(tmp)),
                        declare: false,
                    };
                    stmts.push(Statement {
                        kind: borrow,
                        span: s.span,
                    });
                    let mut ops = vec![Operand::Move(Place::local(r))];
                    ops.extend(args.iter().map(|o| self.operand(o)));
                    let call = StmtKind::Assign {
                        place: place.clone(),
                        value: Rvalue::Call(Operand::Const(crate::mir::Const::Receiver(kind)), ops),
                        declare: *declare,
                    };
                    stmts.push(Statement {
                        kind: call,
                        span: s.span,
                    });
                    stmts.push(Statement {
                        kind: StmtKind::Dead(r),
                        span: s.span,
                    });
                    if self.needs_drop(&t) {
                        self.drop_glue(&t);
                        let kind = StmtKind::Drop(Place::local(tmp), t);
                        stmts.push(Statement { kind, span: s.span });
                    }
                    stmts.push(Statement {
                        kind: StmtKind::Dead(tmp),
                        span: s.span,
                    });
                    continue;
                }
                let value = match value {
                    // `l < r` of what isn't an integer is `Ord`'s method, of
                    // `&l` and `&r`, in temporaries of their own.
                    Rvalue::Binary(_, Operand::Copy(l), Operand::Copy(r), id)
                        if self.m.info.ord_calls.contains(id) =>
                    {
                        let Some(Instance::Fn(symbol, args)) =
                            self.m.info.instances.get(id).cloned()
                        else {
                            unreachable!("a comparison calls an `Ord` method")
                        };
                        let args: Vec<Type> = args
                            .iter()
                            .map(|a| self.subst_inferred(a).erased())
                            .collect();
                        let r_ty = Type::Ref(Region::Erased, false, Box::new(args[0].clone()));
                        let mut temps = Vec::new();
                        for p in [l, r] {
                            out.locals.push(LocalDecl {
                                ty: r_ty.clone(),
                                kind: LocalKind::Temp,
                                mutable: true,
                                name: None,
                                span: s.span,
                                mut_span: None,
                                by_ref: None,
                                deferred: false,
                            });
                            let t = Local(out.locals.len() - 1);
                            let kind = StmtKind::Assign {
                                place: Place::local(t),
                                value: Rvalue::Ref(0, false, p.clone()),
                                declare: false,
                            };
                            stmts.push(Statement { kind, span: s.span });
                            temps.push(t);
                        }
                        let f = self.fn_const(&symbol, args);
                        let call = Rvalue::Call(
                            Operand::Const(f),
                            temps
                                .iter()
                                .map(|t| Operand::Move(Place::local(*t)))
                                .collect(),
                        );
                        let kind = StmtKind::Assign {
                            place: place.clone(),
                            value: call,
                            declare: *declare,
                        };
                        stmts.push(Statement { kind, span: s.span });
                        for t in temps.into_iter().rev() {
                            let kind = StmtKind::Dead(t);
                            stmts.push(Statement { kind, span: s.span });
                        }
                        continue;
                    }
                    Rvalue::Binary(op, l, r, id) => {
                        Rvalue::Binary(*op, self.operand(l), self.operand(r), *id)
                    }
                    Rvalue::Use(o) => Rvalue::Use(self.operand(o)),
                    Rvalue::Ref(..) => value.clone(),
                    Rvalue::Unary(op, o, id) => Rvalue::Unary(*op, self.operand(o), *id),
                    Rvalue::Cast(o, t, id) => {
                        Rvalue::Cast(self.operand(o), self.subst_inferred(t).erased(), *id)
                    }
                    Rvalue::Aggregate(k, t, os) => {
                        if let Agg::Closure(id) = k {
                            self.closure(*id, root);
                        }
                        let t = self.subst_inferred(t).erased();
                        self.copies_in(&t);
                        let os = os.iter().map(|o| self.operand(o)).collect();
                        Rvalue::Aggregate(*k, t, os)
                    }
                    Rvalue::Call(f, args) => Rvalue::Call(
                        self.operand(f),
                        args.iter().map(|o| self.operand(o)).collect(),
                    ),
                };
                let kind = StmtKind::Assign {
                    place: place.clone(),
                    value,
                    declare: *declare,
                };
                stmts.push(Statement { kind, span: s.span });
            }
            blocks.push(Block {
                stmts,
                term: block.term.clone(),
            });
        }
        out.blocks = blocks;
        for r in &mut out.refinements {
            r.pred = *self.pred(&r.pred);
        }
        tidy(&mut out);
        out
    }

    /// The type of what's at `place`, in a copy whose locals are `locals`.
    fn place_ty(&mut self, locals: &[crate::mir::LocalDecl], place: &crate::mir::Place) -> Type {
        use crate::mir::Proj;
        let mut t = locals[place.local.0].ty.clone();
        for p in &place.proj {
            t = match (p, &t) {
                (Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => (**inner).clone(),
                (Proj::Field(_), Type::Array(elem, _)) => (**elem).clone(),
                (Proj::Field(i), Type::Adt(symbol, ..)) if crate::types::is_closure(symbol) => {
                    self.m.info.traits.closures[symbol].fields_at(&t)[*i].clone()
                }
                (Proj::Field(i), _) => self.field_types(&t)[*i].clone(),
                (Proj::Variant(v, i), _) => {
                    self.variant_types(&t).expect("an enum")[*v][*i].clone()
                }
                (p, t) => unreachable!("{p:?} of `{t}`"),
            }
            .erased();
        }
        t
    }

    /// Makes the copies of the structs and enums in `t`, which has no type
    /// parameters left in it.
    fn copies_in(&mut self, t: &Type) {
        if let Type::Adt(name, _, args) = t
            && !crate::types::is_closure(name)
        {
            self.m
                .type_copy(name, args.iter().map(Type::erased).collect());
        }
        for p in t.parts() {
            self.copies_in(p);
        }
    }

    /// Makes the copy of the closure made by the lambda with node `id`, in
    /// the copy of a function `root`, if it isn't made yet.
    fn closure(&mut self, id: NodeId, root: &str) {
        let name = closure_copy(root, id);
        if self.m.bodies.contains_key(&name) {
            return;
        }
        let b = self.m.info.mir[&crate::mir::Key::Closure(id)].clone();
        let b = self.body(&b, root);
        self.m.bodies.insert(name, b);
    }

    /// Operand `o`, in the copy.
    fn operand(&mut self, o: &crate::mir::Operand) -> crate::mir::Operand {
        use crate::mir::{Const, Operand};
        match o {
            Operand::Const(Const::Fn(symbol, args)) => {
                let args = args
                    .iter()
                    .map(|a| self.subst_inferred(a).erased())
                    .collect();
                Operand::Const(self.fn_const(symbol, args))
            }
            Operand::Const(Const::Int(n, t)) => Operand::Const(Const::Int(*n, self.subst(t))),
            Operand::Const(Const::Param(name)) => match self.subst(&Type::Param(name.clone())) {
                Type::Nat(n) => Operand::Const(Const::Int(
                    i128::from(n),
                    Type::Int(crate::types::IntTy::Usize),
                )),
                t => unreachable!("const parameter `{name}` as `{t}`"),
            },
            o => o.clone(),
        }
    }

    /// Function `symbol` at type arguments `args`, which have no type
    /// parameters left in them, as a value: its copy, or the impl's method,
    /// for a trait's (see `target`), or the glue a built-in or derived
    /// `eq` or `discard` is (see `glue.rs`).
    fn fn_const(&mut self, symbol: &str, args: Vec<Type>) -> crate::mir::Const {
        use crate::mir::Const;
        let method = self.m.fns.get(symbol).is_some_and(|d| d.self_ty.is_some());
        if args.is_empty() && !method {
            return Const::Fn(symbol.to_string(), Vec::new());
        }
        match self.target(symbol, args) {
            Target::Fn(of, args, copy) => Const::Fn(self.m.named_copy(&of, args, copy), Vec::new()),
            Target::Eq(t) => Const::Fn(self.eq_glue(&t), Vec::new()),
            Target::Discard(t) => Const::Fn(self.drop_glue(&t), Vec::new()),
        }
    }
}

/// Leaves out of a copy what was there only for drops that do nothing at
/// its types, and which monomorphisation left out: a branch on a drop flag
/// to a block that does nothing, the blocks control no longer gets to, and
/// the flags nothing reads. None of that does anything: a branch's operand
/// is a place's value, and a flag is given a constant.
fn tidy(body: &mut crate::mir::Body) {
    use crate::mir::{Bb, Const, Operand, Rvalue, StmtKind, Term};
    // A branch to a block that does nothing but go where the other goes.
    for b in 0..body.blocks.len() {
        if let Some(Term::Branch(_, t, e)) = &body.blocks[b].term {
            let empty = |x: &Bb, y: &Bb| {
                let block = &body.blocks[x.0];
                block.stmts.is_empty() && matches!(block.term, Some(Term::Goto(g)) if g == *y)
            };
            if empty(t, e) || t == e {
                body.blocks[b].term = Some(Term::Goto(*e));
            }
        }
        // A `match` of an open drop each of whose arms does nothing.
        if let Some(Term::Match(_, _, ts)) = &body.blocks[b].term {
            let to = |t: &Bb| {
                let block = &body.blocks[t.0];
                match (&block.term, block.stmts.is_empty()) {
                    (Some(Term::Goto(g)), true) => Some(*g),
                    _ => None,
                }
            };
            let first = ts.first().and_then(to);
            if first.is_some() && ts.iter().all(|t| to(t) == first) {
                body.blocks[b].term = first.map(Term::Goto);
            }
        }
    }
    let mut seen = vec![false; body.blocks.len()];
    let mut todo = vec![0];
    while let Some(b) = todo.pop() {
        if !std::mem::replace(&mut seen[b], true) {
            todo.extend(body.successors(Bb(b)).iter().map(|s| s.0));
        }
    }
    for (b, seen) in seen.iter().enumerate() {
        if !seen {
            body.blocks[b] = Default::default();
        }
    }
    // The locals read: by an operand, a borrow, a `match` or a branch.
    let mut read = vec![false; body.locals.len()];
    for block in &body.blocks {
        for s in &block.stmts {
            if let StmtKind::Assign { place, value, .. } = &s.kind {
                if !place.proj.is_empty() {
                    read[place.local.0] = true;
                }
                if let Rvalue::Ref(_, _, p) = value {
                    read[p.local.0] = true;
                }
                for p in value.operands().into_iter().filter_map(Operand::place) {
                    read[p.local.0] = true;
                }
            } else if let StmtKind::Inspect(p)
            | StmtKind::Consume(p, _)
            | StmtKind::Kept(p)
            | StmtKind::Drop(p, _)
            | StmtKind::Free(p) = &s.kind
            {
                read[p.local.0] = true;
            } else if let StmtKind::Refine(l, _) = &s.kind {
                read[l.0] = true;
            }
        }
        match &block.term {
            Some(Term::Branch(o, ..)) => {
                if let Some(p) = o.place() {
                    read[p.local.0] = true;
                }
            }
            Some(Term::Match(p, ..)) => read[p.local.0] = true,
            _ => {}
        }
    }
    for block in &mut body.blocks {
        block.stmts.retain(|s| match &s.kind {
            StmtKind::Assign {
                place,
                value: Rvalue::Use(Operand::Const(Const::Bool(_))),
                ..
            } => !place.proj.is_empty() || read[place.local.0] || place.local.0 == 0,
            _ => true,
        });
    }
    let written = |l: usize, body: &crate::mir::Body| {
        body.blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .any(|s| matches!(&s.kind, StmtKind::Assign { place, .. } if place.local.0 == l))
    };
    let gone: Vec<usize> = (1 + body.params..body.locals.len())
        .filter(|&l| !read[l] && !written(l, body))
        .collect();
    for block in &mut body.blocks {
        block
            .stmts
            .retain(|s| !matches!(&s.kind, StmtKind::Dead(l) if gone.contains(&l.0)));
    }
}

/// `t` without its refinement, if it has one: the type it refines.
pub(crate) fn unrefined(t: &TypeExpr) -> &TypeExpr {
    match &t.kind {
        TypeExprKind::Refined(_, inner, _) => inner,
        _ => t,
    }
}

/// How region `r` is written: `'a`, or `'_` for one nobody named.
fn region_name(r: &Region) -> String {
    match r {
        Region::Named(n) if !r.is_elided() && !n.starts_with("'^") => n.clone(),
        Region::Static => "'static".into(),
        _ => "'_".into(),
    }
}

/// The names a monomorphised item uses: of functions, structs and enums.
mod used {
    use crate::ast::*;
    use std::collections::HashSet;

    pub(super) fn in_type_decl(d: &TypeDecl, out: &mut HashSet<String>) {
        match d {
            TypeDecl::Struct(s) => s.fields.iter().for_each(|f| in_type(&f.ty, out)),
            TypeDecl::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| &v.fields)
                .for_each(|t| in_type(t, out)),
            TypeDecl::Alias(a) => in_type(&a.ty, out),
        }
    }

    /// What MIR body `b` uses: the functions it names, and the structs
    /// and enums of its types.
    pub(super) fn in_body(b: &crate::mir::Body, out: &mut HashSet<String>) {
        use crate::mir::{Const, Operand, Rvalue, StmtKind};
        for l in &b.locals {
            in_ty(&l.ty, out);
        }
        for s in b.blocks.iter().flat_map(|b| &b.stmts) {
            let StmtKind::Assign { value, .. } = &s.kind else {
                continue;
            };
            if let Rvalue::Aggregate(_, t, _) | Rvalue::Cast(_, t, _) = value {
                in_ty(t, out);
            }
            for o in value.operands() {
                if let Operand::Const(Const::Fn(name, args)) = o {
                    out.insert(name.clone());
                    args.iter().for_each(|t| in_ty(t, out));
                }
            }
        }
    }

    /// The structs and enums of `t`, by the names of their copies.
    fn in_ty(t: &crate::types::Type, out: &mut HashSet<String>) {
        if let crate::types::Type::Adt(name, _, args) = t {
            out.insert(super::mangle(name, args));
        }
        for p in t.parts() {
            in_ty(p, out);
        }
    }

    pub(super) fn in_fn(f: &FnDecl, out: &mut HashSet<String>) {
        f.params.iter().for_each(|p| in_type(&p.ty, out));
        in_type(&f.ret, out);
    }

    fn in_type(t: &TypeExpr, out: &mut HashSet<String>) {
        match &t.kind {
            TypeExprKind::Named(name, args) => {
                out.insert(name.clone());
                args.iter().for_each(|a| in_type(a, out));
            }
            TypeExprKind::Fn(params, ret, effects) => {
                params.iter().for_each(|p| in_type(p, out));
                in_type(ret, out);
                effects
                    .iter()
                    .flat_map(EffectSet::types)
                    .for_each(|t| in_type(t, out));
            }
            TypeExprKind::Ref(_, _, inner) | TypeExprKind::Slice(inner) => in_type(inner, out),
            TypeExprKind::Proj(inner, ..) | TypeExprKind::Refined(_, inner, _) => {
                in_type(inner, out)
            }
            TypeExprKind::Array(inner, n) => {
                in_type(inner, out);
                in_type(n, out);
            }
            TypeExprKind::Impl(_)
            | TypeExprKind::Unit
            | TypeExprKind::Never
            | TypeExprKind::Lifetime(_)
            | TypeExprKind::Nat(_) => {}
        }
    }
}
