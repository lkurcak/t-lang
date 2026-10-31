//! Modules: which item each path names (step 6d, see docs/06d-modules.md).
//!
//! A module is a namespace: the items declared in it, and those its `use`s
//! bring in. It's *hermetic*: nothing of the modules around it is in scope
//! in it, so `helper` in `mod m` never means the crate root's `helper`,
//! which it can name as `super::helper` or `crate::helper`. So a module
//! means the same wherever it sits.
//!
//! Each module has two namespaces: *types*, for structs, enums, traits and
//! modules, and *values*, for functions. A name can be in both, like a
//! module `main` and a function `main`.
//!
//! An item is visible in the module it's declared in and in the modules
//! in that; if it's `pub`, anywhere in its package; and if it's `export`,
//! in other packages too (see docs/24-visibility.md). A path is resolved
//! segment by segment, each of which must be visible where the path is
//! written. A `use` binds a name like an item declared there, private
//! unless it's `pub use` or `export use`, and can't make more of what it
//! names than it is. What's exported must be reachable from its package's
//! root through what's exported alone (`exports_reachable`), so a
//! package's interface is what its root says.
//!
//! Resolving is a function of the program, not of the order of anything
//! (lemma M1 of the docs): with no glob imports, a name in a module is one
//! item or one `use`, or two in different namespaces. A `use` can name a
//! path through another `use`, so they're resolved on demand, and one that
//! needs itself is an error, so resolving ends.
//!
//! This runs before checking, and leaves what it learns in the program as
//! well as in `Modules`: the parser can't tell `a::f(1)`, a call of a
//! function in a module, from `Shape::Dot(p)`, a variant, nor `a::Point`
//! from the projection `T::Item`, so those are rewritten to say which they
//! are. A trait is named by its *symbol* from here on: `Show` at the crate
//! root, `shapes::Show` in a module, so every reference to a trait,
//! whatever path it was written as, says which one it is.
//!
//! The core library is a package of its own (step 6e, see
//! docs/06e-core.md): a module tree beside the crate root's, whose root is
//! `core`. A path can start with `core` anywhere, as it can with `crate`,
//! which in the core library means its root. Neither package's modules are
//! in the other's: `super` can't leave a package, and its private items are
//! its own. The alloc library is a package the same way, whose root is
//! `alloc`, and which uses the core library's. Every module also sees the
//! *prelude*, what `core::prelude`
//! brings in, after its own items and `use`s: a name a module declares or
//! imports itself always wins, so adding to the prelude never changes what
//! a program means.
//!
//! Two expressions are only sugar, which this rewrites into what they mean,
//! naming the core library's items by their symbols, so that a program's
//! own `Option` can't change them (lemma C4 of the docs): `a..b` is
//! `core::ops::Range::new(a, b)`, and a `for` loop is a `loop` (see
//! `Rewrite::for_loop`).

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Lint, Span, Warning};
use crate::traits::{COPY, DISCARD, EQ};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

/// An item, by where it is in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Item {
    /// By its index in `Program::mods`.
    Mod(usize),
    /// By its index in `Program::types`.
    Type(usize),
    /// By its index in `Program::traits`.
    Trait(usize),
    /// A top-level function, by its `FnId`: its index in `Program::all_fns`.
    Fn(usize),
}

/// A name in a module's namespace: an item declared there, or brought in
/// by a `use`.
#[derive(Debug, Clone)]
pub(crate) struct Binding {
    pub(crate) item: Item,
    /// The module it's declared or imported in.
    pub(crate) module: usize,
    /// How far it's seen from there.
    pub(crate) level: Level,
    /// The `use` it's from, by its index in `Modules::imports`, if it is.
    pub(crate) import: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Module {
    pub(crate) parent: Option<usize>,
    /// What the symbols of the items in it start with: nothing at the crate
    /// root, `a::b::` in `mod b` in `mod a`.
    pub(crate) prefix: String,
    /// Its path from the crate root, for messages: `a::b`.
    pub(crate) path: String,
    pub(crate) types: HashMap<String, Binding>,
    pub(crate) values: HashMap<String, Binding>,
}

/// Which `let` an `else` arm made up by `Rewrite::if_let` is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElseOf {
    /// An `if let`, and whether it has an `else` written.
    IfLet(bool),
    WhileLet,
    LetElse,
}

impl ElseOf {
    /// What it is, for messages: "`if let`".
    pub(crate) fn what(self) -> &'static str {
        match self {
            ElseOf::IfLet(_) => "`if let`",
            ElseOf::WhileLet => "`while let`",
            ElseOf::LetElse => "`let .. else`",
        }
    }
}

/// A name a `use` brings in, and whether anything uses it.
#[derive(Clone, Debug)]
pub(crate) struct Import {
    pub(crate) name: Ident,
    /// The whole `use`, for a fix that removes it, if it brings in only
    /// this.
    pub(crate) alone: Option<Span>,
    /// How far it's seen: a `pub use` or `export use` is for other
    /// modules to use.
    pub(crate) level: Level,
    pub(crate) used: Cell<bool>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Modules {
    pub(crate) mods: Vec<Module>,
    /// What each path with a `::` refers to, by its node: one to a type
    /// where a type is written (`a::Point`, by the type), in a struct
    /// literal or pattern (by its name), and one to a function where one is
    /// used (by the expression).
    pub(crate) paths: HashMap<NodeId, Item>,
    /// Where each `?` is, which is where the calls it means are
    /// (`Rewrite::try_expr`), for saying what's wrong with them in terms of
    /// the `?`.
    pub(crate) tries: HashSet<Span>,
    /// The `else` arm of the `match` each `if let`, `while let` and `let ..
    /// else` means, by where its pattern is, the `let` (`Rewrite::if_let`):
    /// for saying what's wrong with it in terms of what was written.
    pub(crate) elses: HashMap<Span, ElseOf>,
    /// The pattern, `_`, of the `let _: ! = ..` each `let .. else` puts its
    /// `else` in, for saying what's wrong if it can finish.
    pub(crate) must_leave: HashSet<NodeId>,
    pub(crate) imports: Vec<Import>,
    /// The symbol of each trait, by its index in `Program::traits`.
    pub(crate) trait_symbols: Vec<String>,
    /// Where each module ends: before its `}`, or at the end of the file.
    pub(crate) ends: Vec<usize>,
    /// The last segment of the path of each name a `use` brings in, and
    /// what it names: something in each namespace.
    pub(crate) leaf_items: Vec<(Ident, Vec<Item>)>,
    /// The first segment of each path that names a module, and the module,
    /// for hovers. Not the segments after it, whose spans aren't kept.
    pub(crate) module_refs: Vec<(Span, usize)>,
    /// The core library's root module, and its prelude.
    pub(crate) core: Option<usize>,
    /// The root modules of the libraries built into the compiler, as
    /// `Program::libraries`.
    pub(crate) libraries: Vec<usize>,
    /// The packages each of the program's packages depends on, by the
    /// root module of each, as `Program::packages` has them: by name, the
    /// first segment of a path in it.
    pub(crate) deps: HashMap<usize, Vec<(String, usize)>>,
    pub(crate) prelude: Option<usize>,
    /// Warnings about what's rewritten here: empty ranges.
    pub(crate) warnings: Vec<Warning>,
    /// Where each item is declared, and how far it's seen from there.
    decls: HashMap<Item, (usize, Level)>,
    /// Each name each `use` brings in, and how far resolving it has got:
    /// a later package's paths can go through an earlier one's `pub use`.
    leaves: Vec<Leaf>,
    states: Vec<State>,
}

/// Which namespace a name is looked up in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ns {
    Types,
    Values,
}

impl Modules {
    /// The binding of `name` in module `m`'s namespace `ns`, noting that
    /// its `use`, if it has one, is used.
    fn get(&self, m: usize, name: &str, ns: Ns) -> Option<&Binding> {
        let table = |m: usize| match ns {
            Ns::Types => &self.mods[m].types,
            Ns::Values => &self.mods[m].values,
        };
        let b = table(m)
            .get(name)
            .or_else(|| table(self.prelude?).get(name))?;
        if let Some(i) = b.import {
            self.imports[i].used.set(true);
        }
        Some(b)
    }

    /// What type name `name` means at the level of module `m`: the struct
    /// or enum it's declared or imported as there, if it is.
    pub(crate) fn type_named(&self, m: usize, name: &str) -> Option<usize> {
        match self.get(m, name, Ns::Types)?.item {
            Item::Type(k) => Some(k),
            _ => None,
        }
    }

    /// What function name `name` means at the level of module `m`, if
    /// anything.
    pub(crate) fn fn_named(&self, m: usize, name: &str) -> Option<usize> {
        match self.get(m, name, Ns::Values)?.item {
            Item::Fn(i) => Some(i),
            _ => None,
        }
    }

    /// Is trait `symbol` in scope in module `m`: declared or imported
    /// there, or brought in by the prelude?
    pub(crate) fn trait_in_scope(&self, m: usize, symbol: &str) -> bool {
        self.trait_binding(m, symbol).is_some()
    }

    /// The binding that puts trait `symbol` in scope in module `m`, if any:
    /// its own, or else the prelude's.
    fn trait_binding(&self, m: usize, symbol: &str) -> Option<&Binding> {
        let of = |m: usize| {
            self.mods[m]
                .types
                .values()
                .find(|b| matches!(b.item, Item::Trait(k) if self.trait_symbols[k] == symbol))
        };
        of(m).or_else(|| of(self.prelude?))
    }

    /// Notes that trait `symbol` was used in module `m`: its `use`, if
    /// that's how it's in scope.
    pub(crate) fn use_trait(&self, m: usize, symbol: &str) {
        if let Some(i) = self.trait_binding(m, symbol).and_then(|b| b.import) {
            self.imports[i].used.set(true);
        }
    }

    /// The shortest path to trait `symbol` from module `m`, for a `use`
    /// that brings it in: its symbol from the crate root, which is a path
    /// from it, or else that after `crate::`.
    pub(crate) fn path_to_trait(&self, m: usize, symbol: &str) -> String {
        match m {
            _ if is_library_symbol(symbol) => symbol.to_string(),
            ROOT => symbol.to_string(),
            _ => format!("crate::{symbol}"),
        }
    }

    /// The path of the core library's public function `name`, if there's
    /// exactly one, in a module of its own that's public: `core::mem::swap`
    /// for `swap`.
    pub(crate) fn core_fn(&self, name: &str) -> Option<String> {
        let core = self.core?;
        let found: Vec<&Module> = self
            .mods
            .iter()
            .enumerate()
            .filter(|&(m, _)| m != core && self.in_core(m))
            .map(|(_, module)| module)
            .filter(|module| {
                module
                    .values
                    .get(name)
                    .is_some_and(|b| b.level == Level::Export && b.import.is_none())
            })
            .collect();
        match found[..] {
            [module] => Some(format!("{}{name}", module.prefix)),
            _ => None,
        }
    }

    /// The path of the public struct or enum `name` of a library built into
    /// the compiler, if there's exactly one, declared where it is:
    /// `alloc::boxed::Box` for `Box`, `std::Os` for `Os`.
    pub(crate) fn library_type(&self, name: &str) -> Option<String> {
        let found: Vec<&Module> = (self.mods.iter().enumerate())
            .filter(|&(m, _)| self.in_library(m))
            .map(|(_, module)| module)
            .filter(|module| {
                module.types.get(name).is_some_and(|b| {
                    b.level == Level::Export
                        && b.import.is_none()
                        && matches!(b.item, Item::Type(_))
                })
            })
            .collect();
        match found[..] {
            [module] => Some(format!("{}{name}", module.prefix)),
            _ => None,
        }
    }

    /// The root of the package module `m` is in: the crate root, or the
    /// core library's.
    pub(crate) fn package_root(&self, mut m: usize) -> usize {
        while let Some(p) = self.mods[m].parent {
            m = p;
        }
        m
    }

    /// Is module `m` in the core library?
    pub(crate) fn in_core(&self, m: usize) -> bool {
        Some(self.package_root(m)) == self.core
    }

    /// Is module `m` in a library built into the compiler, the core
    /// library or the alloc library?
    pub(crate) fn in_library(&self, m: usize) -> bool {
        self.libraries.contains(&self.package_root(m))
    }

    /// The root module of the library `name`, if it's one built in, as a
    /// path can start with: `core` or `alloc`.
    pub(crate) fn library(&self, name: &str) -> Option<usize> {
        (self.libraries.iter().copied()).find(|&m| self.mods[m].path == name)
    }

    /// The root module of the package called `name` where module `from`
    /// is, as a path there can start with: a library built in, or one its
    /// package's `t.toml` names. Not one of those's: what a package
    /// depends on is its own business.
    pub(crate) fn package_named(&self, from: usize, name: &str) -> Option<usize> {
        let deps = self.deps.get(&self.package_root(from));
        let dep = deps.and_then(|d| d.iter().find(|(n, _)| n == name));
        self.library(name).or(dep.map(|&(_, m)| m))
    }

    /// Is `inner` module `outer` or in it?
    pub(crate) fn within(&self, inner: usize, outer: usize) -> bool {
        let mut m = Some(inner);
        while let Some(i) = m {
            if i == outer {
                return true;
            }
            m = self.mods[i].parent;
        }
        false
    }

    /// Can code in module `from` see something declared in module
    /// `module`, which is seen as far as `level`?
    pub(crate) fn visible(&self, module: usize, level: Level, from: usize) -> bool {
        match level {
            Level::Module => self.within(from, module),
            Level::Package => self.package_root(from) == self.package_root(module),
            Level::Export => true,
        }
    }

    /// Why something declared in `module`, seen as far as `level`, can't be
    /// seen somewhere, for messages: "is private to `a::b`", "isn't
    /// exported by `core`".
    pub(crate) fn hidden_by(&self, module: usize, level: Level) -> String {
        match level {
            Level::Module => format!("is private to {}", self.describe(module)),
            _ => format!("isn't exported by {}", self.describe_package(module)),
        }
    }

    /// Is module `m` in a package the program depends on?
    pub(crate) fn in_dependency(&self, m: usize) -> bool {
        let root = self.package_root(m);
        root != ROOT && !self.libraries.contains(&root)
    }

    /// The package module `m` is in, for messages: "`core`", "the crate".
    pub(crate) fn describe_package(&self, m: usize) -> String {
        match self.package_root(m) {
            ROOT => "the crate".into(),
            root => format!("`{}`", self.mods[root].path),
        }
    }

    /// `m` for messages: "`a::b`", or "the crate root".
    pub(crate) fn describe(&self, m: usize) -> String {
        match m {
            ROOT => "the crate root".into(),
            m => format!("`{}`", self.mods[m].path),
        }
    }

    /// Module `m`'s path as code would write it from anywhere: `crate`,
    /// `crate::a::b`, `core`, `alloc::list`.
    pub(crate) fn full_path(&self, m: usize) -> String {
        // A library's start with its name already, as a dependency's do.
        match (self.package_root(m) != ROOT, self.mods[m].path.as_str()) {
            (true, path) => path.to_string(),
            (false, "") => "crate".to_string(),
            (false, path) => format!("crate::{path}"),
        }
    }
}

/// The symbol of trait `t`: its name, after its module's path if it's not
/// at the crate root.
fn trait_symbol(mods: &[Module], t: &TraitDecl) -> String {
    format!("{}{}", mods[t.module].prefix, t.name.name)
}

/// Resolves the paths of the package of `prog` whose items come after
/// `from` (see `Counts`), rewriting them as the module docs say, in
/// `modules`, what resolving those before left. Theirs stay as they are.
pub(crate) fn resolve(
    prog: &mut Program,
    modules: &mut Modules,
    from: Counts,
) -> Result<(), Error> {
    let mut r = Resolver {
        m: std::mem::take(modules),
    };
    let result = r.package(prog, from);
    *modules = r.m;
    result
}

impl Resolver {
    fn package(&mut self, prog: &mut Program, from: Counts) -> Result<(), Error> {
        let r = self;
        for (i, m) in prog.mods.iter().enumerate().skip(r.m.mods.len()) {
            let (prefix, path) = match m.parent {
                None => (String::new(), String::new()),
                Some(p) => {
                    let path = match p {
                        ROOT => m.name.name.clone(),
                        p => format!("{}::{}", r.m.mods[p].path, m.name.name),
                    };
                    (format!("{path}::"), path)
                }
            };
            debug_assert_eq!(i, r.m.mods.len());
            // A library's root is a package's: no module is around it.
            let parent = match prog.is_package_root(i) {
                true => None,
                false => m.parent,
            };
            r.m.mods.push(Module {
                parent,
                prefix,
                path,
                ..Module::default()
            });
        }
        r.m.core = prog.core;
        r.m.libraries = prog.libraries.clone();
        r.m.deps = (prog.packages.iter())
            .map(|p| (p.root, p.deps.clone()))
            .collect();
        r.declare(prog, from)?;
        r.imports(prog, from)?;
        if r.m.prelude.is_none() {
            r.m.prelude = prog
                .core
                .and_then(|c| match r.m.mods[c].types.get("prelude") {
                    Some(Binding {
                        item: Item::Mod(p), ..
                    }) => Some(*p),
                    _ => None,
                });
        }
        let symbols: Vec<String> = prog.traits[from.traits..]
            .iter()
            .map(|t| trait_symbol(&r.m.mods, t))
            .collect();
        r.m.ends = prog
            .mods
            .iter()
            .map(|m| match m.parent {
                None => m.span.end,
                Some(_) => m.span.end - 1,
            })
            .collect();
        r.m.trait_symbols.extend(symbols.iter().cloned());
        let all = r.m.trait_symbols.clone();
        let variants = prog
            .types
            .iter()
            .map(|d| match d {
                TypeDecl::Enum(d) => Some(
                    d.variants
                        .iter()
                        .map(|v| (v.name.name.clone(), v.fields.len()))
                        .collect(),
                ),
                TypeDecl::Struct(_) | TypeDecl::Alias(_) => None,
            })
            .collect();
        let mut w = Rewrite {
            r,
            symbols: &all,
            module: ROOT,
            params: Vec::new(),
            interface: None,
            next_id: prog.next_id,
            variants,
            in_pred: false,
        };
        let rewritten = w.program(prog, from);
        // What's rewritten stays rewritten, ids and all, even past an error.
        prog.next_id = w.next_id;
        rewritten?;
        for (t, symbol) in prog.traits[from.traits..].iter_mut().zip(symbols) {
            t.name.name = symbol;
        }
        r.exports_reachable(prog, from)
    }

    /// Checks that each item of the package that's exported can be named
    /// by other packages: that some path from its package's root reaches
    /// it through exported modules and `export use`s alone. So what a
    /// package exports is what's reachable that way, which is what its
    /// root says, and nothing exported is hidden where only its own
    /// modules see it (see docs/24-visibility.md).
    fn exports_reachable(&self, prog: &Program, from: Counts) -> Result<(), Error> {
        let mut items: Vec<(Item, usize, &Ident, &str)> = Vec::new();
        for (i, m) in prog.mods.iter().enumerate().skip(from.mods.max(1)) {
            if m.vis.is_export() && !prog.is_package_root(i) {
                let parent = m.parent.expect("not a root");
                items.push((Item::Mod(i), parent, &m.name, "module "));
            }
        }
        for (k, d) in prog.types.iter().enumerate().skip(from.types) {
            if d.vis().is_export() {
                items.push((Item::Type(k), d.module(), d.name(), ""));
            }
        }
        for (k, t) in prog.traits.iter().enumerate().skip(from.traits) {
            if t.vis.is_export() {
                items.push((Item::Trait(k), t.module, &t.name, "trait "));
            }
        }
        let fns = prog.all_fns().into_iter().enumerate().skip(from.all_fns);
        for (i, (f, parent, method)) in fns {
            if f.vis.is_export() && parent.is_none() && method.is_none() {
                items.push((Item::Fn(i), f.module, &f.name, ""));
            }
        }
        items.sort_by_key(|(_, _, name, _)| name.span.start);
        let mut reached: HashSet<Item> = HashSet::new();
        let mut done: HashSet<usize> = HashSet::new();
        for &(_, module, ..) in &items {
            let mut todo = vec![self.m.package_root(module)];
            while let Some(m) = todo.pop() {
                if !done.insert(m) {
                    continue;
                }
                let table = self.m.mods[m]
                    .types
                    .values()
                    .chain(self.m.mods[m].values.values());
                for b in table.filter(|b| b.level == Level::Export) {
                    reached.insert(b.item);
                    if let Item::Mod(inner) = b.item {
                        todo.push(inner);
                    }
                }
            }
        }
        let Some(&(_, module, name, what)) =
            items.iter().find(|(item, ..)| !reached.contains(item))
        else {
            return Ok(());
        };
        // The outermost module on the way that isn't exported.
        let mut hidden = module;
        let mut m = Some(module);
        while let Some(i) = m {
            if self.m.mods[i].parent.is_some() && !prog.mods[i].vis.is_export() {
                hidden = i;
            }
            m = self.m.mods[i].parent;
        }
        let hidden = self.m.mods[hidden]
            .path
            .rsplit("::")
            .next()
            .unwrap_or_default();
        Err(Error::ty(
            name.span,
            format!(
                "{what}`{}` is exported, but other packages can't name it: module `{hidden}` isn't \
                 exported, and no `export use` names it",
                name.name
            ),
        ))
    }
}

/// A name a `use` brings in, as written.
#[derive(Clone, Debug)]
struct Leaf {
    module: usize,
    path: Vec<Ident>,
    name: Ident,
    vis: Vis,
    decl: Span,
}

/// How far resolving a `use` has got.
#[derive(Clone, Debug)]
enum State {
    Todo,
    Doing,
    /// What its path names, in each namespace.
    Done(Option<Binding>, Option<Binding>),
}

struct Resolver {
    m: Modules,
}

impl Resolver {
    /// Puts each item in its module's namespace.
    fn declare(&mut self, prog: &Program, from: Counts) -> Result<(), Error> {
        let mut items: Vec<(usize, &Ident, Level, Item, Ns)> = Vec::new();
        for (i, m) in prog.mods.iter().enumerate().skip(from.mods.max(1)) {
            // A library isn't a module of the crate root's.
            if prog.is_package_root(i) {
                continue;
            }
            let parent = m.parent.expect("not the crate root");
            // As a library's name can't be a module's (see `parser.rs`).
            let deps = self.m.deps.get(&self.m.package_root(parent));
            if deps.is_some_and(|d| d.iter().any(|(n, _)| *n == m.name.name)) {
                return Err(Error::syntax(
                    m.name.span,
                    format!(
                        "a module can't be called `{}`: that's the name of a package this one \
                         depends on",
                        m.name.name
                    ),
                ));
            }
            items.push((parent, &m.name, m.vis.level(), Item::Mod(i), Ns::Types));
        }
        for (k, d) in prog.types.iter().enumerate().skip(from.types) {
            items.push((
                d.module(),
                d.name(),
                d.vis().level(),
                Item::Type(k),
                Ns::Types,
            ));
        }
        for (k, t) in prog.traits.iter().enumerate().skip(from.traits) {
            items.push((t.module, &t.name, t.vis.level(), Item::Trait(k), Ns::Types));
        }
        let fns = prog.all_fns().into_iter().enumerate().skip(from.all_fns);
        for (i, (f, ..)) in
            fns.filter(|(_, (_, parent, method))| parent.is_none() && method.is_none())
        {
            items.push((f.module, &f.name, f.vis.level(), Item::Fn(i), Ns::Values));
        }
        items.sort_by_key(|(_, name, ..)| name.span.start);
        for &(module, _, level, item, _) in &items {
            self.m.decls.insert(item, (module, level));
        }
        for (module, name, level, item, ns) in items {
            let binding = Binding {
                item,
                module,
                level,
                import: None,
            };
            let table = match ns {
                Ns::Types => &mut self.m.mods[module].types,
                Ns::Values => &mut self.m.mods[module].values,
            };
            if let Some(old) = table.insert(name.name.clone(), binding) {
                return Err(already(name, old.item, item));
            }
        }
        Ok(())
    }

    /// Resolves every `use`, then adds the names they bring in to their
    /// modules.
    fn imports(&mut self, prog: &Program, from: Counts) -> Result<(), Error> {
        let first = self.m.leaves.len();
        for u in &prog.uses[from.uses..] {
            for (path, name) in u.tree.leaves() {
                self.m.leaves.push(Leaf {
                    module: u.module,
                    path: path.into_iter().cloned().collect(),
                    name: name.clone(),
                    vis: u.vis,
                    decl: u.span,
                });
            }
        }
        self.m.states.resize(self.m.leaves.len(), State::Todo);
        for i in first..self.m.leaves.len() {
            self.leaf(i)?;
        }
        for (i, leaf) in self.m.leaves.iter().enumerate().skip(first) {
            let alone = self.m.leaves.iter().filter(|l| l.decl == leaf.decl).count() == 1;
            self.m.imports.push(Import {
                name: leaf.name.clone(),
                alone: alone.then_some(leaf.decl),
                level: leaf.vis.level(),
                used: Cell::new(false),
            });
            let State::Done(types, values) = &self.m.states[i] else {
                unreachable!("every `use` is resolved");
            };
            let items = [types, values].into_iter().flatten().map(|b| b.item);
            let last = leaf.path[leaf.path.len() - 1].clone();
            self.m.leaf_items.push((last, items.collect()));
            for (target, ns) in [(types, Ns::Types), (values, Ns::Values)] {
                let Some(target) = target else {
                    continue;
                };
                let binding = Binding {
                    item: target.item,
                    module: leaf.module,
                    level: leaf.vis.level(),
                    import: Some(i),
                };
                let module = &mut self.m.mods[leaf.module];
                let table = match ns {
                    Ns::Types => &mut module.types,
                    Ns::Values => &mut module.values,
                };
                let name = &leaf.name;
                match table.get(&name.name) {
                    Some(old) if old.import.is_some() => {
                        return Err(Error::ty(
                            name.span,
                            format!("`{}` is imported twice", name.name),
                        ));
                    }
                    Some(_) => {
                        return Err(Error::ty(
                            name.span,
                            format!("`{}` is already defined in this module", name.name),
                        ));
                    }
                    None => {
                        table.insert(name.name.clone(), binding);
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolves `use` leaf `i`, and those it goes through.
    fn leaf(&mut self, i: usize) -> Result<(Option<Binding>, Option<Binding>), Error> {
        match &self.m.states[i] {
            State::Done(t, v) => return Ok((t.clone(), v.clone())),
            State::Doing => {
                return Err(Error::ty(
                    self.m.leaves[i].decl,
                    "these imports go round in a circle: each needs another to say what it is",
                ));
            }
            State::Todo => {}
        }
        self.m.states[i] = State::Doing;
        let leaf = &self.m.leaves[i];
        let (from, path) = (leaf.module, leaf.path.clone());
        let (last, init) = path.split_last().expect("a path has a segment");
        let module = match init.is_empty() {
            // `use name;` can only be of something in this module already,
            // which it would bring in again.
            true => {
                return Err(Error::ty(
                    last.span,
                    format!(
                        "`use {}` names nothing new; start the path with `crate::`, `super::` \
                         or `self::`, or a module",
                        last.name
                    ),
                ));
            }
            false => self.modules(from, init)?,
        };
        if matches!(last.name.as_str(), "crate" | "super" | "self") {
            return Err(Error::ty(
                last.span,
                format!("`{}` can only start a path", last.name),
            ));
        }
        let types = self.lookup(module, &last.name, Ns::Types)?;
        let values = self.lookup(module, &last.name, Ns::Values)?;
        let seen = |b: &Option<Binding>| {
            b.as_ref()
                .is_some_and(|b| self.m.visible(b.module, b.level, from))
        };
        let (types, values) = match (seen(&types), seen(&values)) {
            (true, true) => (types, values),
            (true, false) => (types, None),
            (false, true) => (None, values),
            (false, false) => {
                let Some(b) = types.as_ref().or(values.as_ref()) else {
                    return Err(no_item(&self.m, module, last));
                };
                return Err(private(&self.m, &last.name, b, last.span));
            }
        };
        let level = self.m.leaves[i].vis.level();
        let below = [&types, &values]
            .into_iter()
            .flatten()
            .find(|b| b.level < level);
        if let Some(b) = below {
            let message = match (level, b.level) {
                (Level::Export, Level::Package) => format!(
                    "`{}` isn't exported, so `export use` can't export it",
                    last.name
                ),
                (Level::Export, _) => format!(
                    "`{}` is private, so `export use` can't export it",
                    last.name
                ),
                _ => format!(
                    "`{}` is private, so `pub use` can't make it public",
                    last.name
                ),
            };
            return Err(Error::ty(last.span, message));
        }
        self.m.states[i] = State::Done(types.clone(), values.clone());
        Ok((types, values))
    }

    /// The binding of `name` in module `m`, while `use`s are being resolved:
    /// an item declared there, or a `use` there, resolved now if it isn't
    /// yet.
    fn lookup(&mut self, m: usize, name: &str, ns: Ns) -> Result<Option<Binding>, Error> {
        let own = match ns {
            Ns::Types => self.m.mods[m].types.get(name),
            Ns::Values => self.m.mods[m].values.get(name),
        };
        if let Some(b) = own {
            return Ok(Some(b.clone()));
        }
        let leaves: Vec<usize> = (0..self.m.leaves.len())
            .filter(|&i| self.m.leaves[i].module == m && self.m.leaves[i].name.name == name)
            .collect();
        for i in leaves {
            let (types, values) = self.leaf(i)?;
            let target = match ns {
                Ns::Types => types,
                Ns::Values => values,
            };
            if let Some(t) = target {
                let level = self.m.leaves[i].vis.level();
                return Ok(Some(Binding {
                    item: t.item,
                    module: m,
                    level,
                    import: Some(i),
                }));
            }
        }
        Ok(None)
    }

    /// The module that `segs`, the start of a path written in module
    /// `from`, names. Each segment after the first must be visible there.
    fn modules(&mut self, from: usize, segs: &[Ident]) -> Result<usize, Error> {
        let mut m = from;
        for (i, seg) in segs.iter().enumerate() {
            m = match seg.name.as_str() {
                "crate" if i == 0 => self.m.package_root(from),
                "self" if i == 0 => from,
                "super" if i == 0 || segs[..i].iter().all(|s| s.name == "super") => {
                    match self.m.mods[m].parent {
                        Some(p) => p,
                        None => {
                            return Err(Error::ty(
                                seg.span,
                                "there's no module around this one: `super` at the crate root",
                            ));
                        }
                    }
                }
                "crate" | "self" | "super" => {
                    return Err(Error::ty(
                        seg.span,
                        format!("`{}` can only start a path", seg.name),
                    ));
                }
                name => {
                    let found = self.lookup(m, name, Ns::Types)?;
                    if found.is_none()
                        && i == 0
                        && let Some(library) = self.m.package_named(from, name)
                    {
                        m = library;
                        self.m.module_refs.push((seg.span, m));
                        continue;
                    }
                    let Some(b) = found else {
                        return Err(match i {
                            0 => Error::ty(seg.span, format!("unknown module `{name}`")),
                            _ => no_item(&self.m, m, seg),
                        });
                    };
                    if i > 0 && !self.m.visible(b.module, b.level, from) {
                        return Err(private(&self.m, name, &b, seg.span));
                    }
                    match b.item {
                        Item::Mod(inner) => {
                            if i == 0 {
                                self.m.module_refs.push((seg.span, inner));
                            }
                            inner
                        }
                        _ => {
                            return Err(Error::ty(seg.span, format!("`{name}` isn't a module")));
                        }
                    }
                }
            };
        }
        Ok(m)
    }
}

/// The error for `name` being declared twice in a module, as `old` and
/// then `new`.
fn already(name: &Ident, old: Item, new: Item) -> Error {
    let message = match (old, new) {
        (Item::Type(_), Item::Type(_)) => format!("type `{}` is already defined", name.name),
        (Item::Trait(_), Item::Trait(_)) => format!("trait `{}` is already defined", name.name),
        (Item::Type(_), Item::Trait(_)) => format!("`{}` is already the name of a type", name.name),
        _ => format!("`{}` is already defined", name.name),
    };
    Error::ty(name.span, message)
}

/// The error for module `m` having no item `name`.
fn no_item(mods: &Modules, m: usize, name: &Ident) -> Error {
    let what = match m {
        ROOT => "the crate root".to_string(),
        m => format!(
            "`{}`",
            mods.mods[m].path.rsplit("::").next().unwrap_or_default()
        ),
    };
    Error::ty(name.span, format!("{what} has no item `{}`", name.name))
}

/// The error for naming `name`, bound as `b`, at `at`, where it can't be
/// seen.
fn private(mods: &Modules, name: &str, b: &Binding, at: Span) -> Error {
    let what = match b.item {
        Item::Mod(_) => format!("module `{name}`"),
        _ => format!("`{name}`"),
    };
    Error::ty(at, format!("{what} {}", mods.hidden_by(b.module, b.level)))
}

/// The error for `export` at `at` on `member`, a field or method of type
/// `of`, which isn't exported: a member is seen no further than its type.
fn not_exported_member(kind: &str, member: &Ident, of: &Ident, at: Span) -> Error {
    Error::ty(
        at,
        format!(
            "{kind} `{}` can't be exported: `{}` isn't",
            member.name, of.name
        ),
    )
    .with_fix(Fix::new(
        "make it `pub`",
        vec![Edit {
            span: at,
            text: "pub".into(),
        }],
    ))
}

/// Rewrites a program's paths, as the module docs say.
struct Rewrite<'r> {
    r: &'r mut Resolver,
    /// The symbol of each trait.
    symbols: &'r [String],
    /// The module of what's being rewritten.
    module: usize,
    /// The type parameters in scope, innermost last: a name among them
    /// isn't a module.
    params: Vec<String>,
    /// While in the interface of a `pub` or `export` item, what to say of a
    /// type or trait in it that isn't seen as far: "`f` is public, but its
    /// signature names", and how far it's seen.
    interface: Option<(String, Level)>,
    /// `Program::next_id`, while the program is being rewritten: the nodes
    /// made here take ids from it.
    next_id: u32,
    /// The variants of each enum, by its index in `Program::types`, by name
    /// and how many fields each has; `None` for a struct or alias.
    variants: Vec<Option<Vec<(String, usize)>>>,
    /// In a refinement's predicate or an `ensures`, where `old(x)` is
    /// the logic's, not a call: only `x` is rewritten.
    in_pred: bool,
}

impl Rewrite<'_> {
    /// An id for a node made here (see `NodeId`).
    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    fn program(&mut self, prog: &mut Program, from: Counts) -> Result<(), Error> {
        for d in &mut prog.types[from.types..] {
            self.module = d.module();
            self.type_decl(d)?;
        }
        for t in &mut prog.traits[from.traits..] {
            self.module = t.module;
            self.params.push("Self".into());
            let interface = interface_of(t.vis, &format!("trait `{}`", t.name.name), "it names");
            self.interface = interface.clone();
            for s in &mut t.supers {
                self.trait_ref(s)?;
            }
            for a in &mut t.assoc {
                for b in &mut a.bounds {
                    self.trait_ref(b)?;
                }
            }
            self.interface = None;
            for m in &mut t.methods {
                self.function(&mut m.decl, interface.clone())?;
            }
            self.params.pop();
        }
        for d in &mut prog.impls[from.impls..] {
            self.module = d.module;
            let params = d.generics.types.len() + 1;
            self.params
                .extend(d.generics.types.iter().map(|p| p.name.clone()));
            self.params.push("Self".into());
            if let Some(t) = &mut d.trait_name {
                self.trait_ref(t)?;
            }
            self.generics(&mut d.generics)?;
            self.ty(&mut d.self_ty)?;
            for a in &mut d.assoc {
                self.ty(&mut a.ty)?;
            }
            if d.trait_name.is_none()
                && let Some(at) = d.methods.iter().find_map(|f| match f.vis {
                    Vis::Export(at) => Some((at, &f.name)),
                    _ => None,
                })
                && let TypeExprKind::Named(ty, _) = &d.self_ty.kind
            {
                let item = match self.r.m.paths.get(&d.self_ty.id) {
                    Some(&item) => Some(item),
                    None => self.r.m.get(self.module, ty, Ns::Types).map(|b| b.item),
                };
                if let Some(item @ Item::Type(_)) = item
                    && self.r.m.decls[&item].1 != Level::Export
                {
                    let ty = Ident {
                        name: ty.rsplit("::").next().unwrap_or(ty).to_string(),
                        span: d.self_ty.span,
                        id: d.self_ty.id,
                    };
                    return Err(not_exported_member("method", at.1, &ty, at.0));
                }
            }
            for f in &mut d.methods {
                let interface = signature(f, d.trait_name.is_none());
                self.function(f, interface)?;
            }
            self.params.truncate(self.params.len() - params);
        }
        for f in &mut prog.fns[from.fns..] {
            self.module = f.module;
            let interface = signature(f, true);
            self.function(f, interface)?;
        }
        Ok(())
    }

    /// What path `name`, written in the current module, names in `ns`: the
    /// module its start names, and the binding of its last segment there.
    fn path(&mut self, name: &Ident, ns: Ns) -> Result<Binding, Error> {
        let segs = segments(name);
        let (last, init) = segs.split_last().expect("a path has a segment");
        let from = self.module;
        let m = self.r.modules(from, init)?;
        let b = match self.r.m.get(m, &last.name, ns) {
            Some(b) => b.clone(),
            None => return Err(no_item(&self.r.m, m, last)),
        };
        if !self.r.m.visible(b.module, b.level, from) {
            return Err(private(&self.r.m, &last.name, &b, name.span));
        }
        Ok(b)
    }

    /// Does `name`, a path in the current module, name a module? Not if
    /// it's a type parameter, or isn't a module in the types namespace.
    fn is_module(&mut self, name: &Ident) -> Result<bool, Error> {
        let segs = segments(name);
        let first = &segs[0].name;
        if self.params.contains(first) {
            return Ok(false);
        }
        if matches!(first.as_str(), "crate" | "super" | "self") {
            return Ok(true);
        }
        let Some(b) = self.r.m.get(self.module, first, Ns::Types) else {
            if self.r.m.package_named(self.module, first).is_none() {
                return Ok(false);
            }
            return match segs.len() {
                1 => Ok(true),
                _ => Ok(matches!(self.path(name, Ns::Types)?.item, Item::Mod(_))),
            };
        };
        if !matches!(b.item, Item::Mod(_)) {
            return Ok(false);
        }
        match segs.len() {
            1 => Ok(true),
            _ => Ok(matches!(self.path(name, Ns::Types)?.item, Item::Mod(_))),
        }
    }

    /// Checks that `item`, written as `name` at `at`, can be in the
    /// interface being walked, if it's a `pub` or `export` item's: that
    /// it's seen as far (lemma M5 of docs/06d-modules.md). Otherwise code
    /// that can use the item couldn't write down what it takes or gives.
    fn exposed(&self, item: Item, name: &str, at: Span) -> Result<(), Error> {
        let Some((interface, want)) = &self.interface else {
            return Ok(());
        };
        let (module, level) = self.r.m.decls[&item];
        if level >= *want {
            return Ok(());
        }
        let name = name.rsplit("::").next().unwrap_or(name);
        let why = match level {
            Level::Module => format!("which is private to {}", self.r.m.describe(module)),
            _ => "which isn't exported".to_string(),
        };
        Err(Error::ty(at, format!("{interface} `{name}`, {why}")))
    }

    /// A reference to a trait, which from here on names it by its symbol.
    fn trait_ref(&mut self, name: &mut Ident) -> Result<(), Error> {
        if name.name.contains("::") {
            let b = self.path(name, Ns::Types)?;
            match b.item {
                Item::Trait(k) => {
                    self.exposed(b.item, &name.name, name.span)?;
                    name.name = self.symbols[k].clone();
                }
                _ => {
                    return Err(Error::ty(
                        name.span,
                        format!("`{}` isn't a trait", name.name),
                    ));
                }
            }
            return Ok(());
        }
        if let Some(b) = self.r.m.get(self.module, &name.name, Ns::Types)
            && let Item::Trait(k) = b.item
        {
            self.exposed(b.item, &name.name, name.span)?;
            name.name = self.symbols[k].clone();
        }
        Ok(())
    }

    /// A path to a struct or enum, written as `name`: noted, if it's
    /// written with a `::`.
    fn type_path(&mut self, name: &Ident) -> Result<(), Error> {
        if !name.name.contains("::") {
            // A type parameter, or `i64`, or a local type, or one of the
            // module's: only the last can be private, and only it can be
            // in the interface of an item of the module.
            if !self.params.contains(&name.name)
                && let Some(b) = self.r.m.get(self.module, &name.name, Ns::Types)
                && let Item::Type(_) = b.item
            {
                self.exposed(b.item, &name.name, name.span)?;
            }
            return Ok(());
        }
        let b = self.path(name, Ns::Types)?;
        match b.item {
            Item::Type(_) => {
                self.exposed(b.item, &name.name, name.span)?;
                self.r.m.paths.insert(name.id, b.item);
                Ok(())
            }
            _ => Err(Error::ty(
                name.span,
                format!("`{}` isn't a struct or enum", name.name),
            )),
        }
    }

    fn type_decl(&mut self, d: &mut TypeDecl) -> Result<(), Error> {
        let n = d.type_params().len();
        self.params
            .extend(d.type_params().iter().map(|p| p.name.clone()));
        let d_vis = d.vis();
        let interface = match self.interface {
            None => interface_of(d_vis, &format!("`{}`", d.name().name), "it names"),
            Some(_) => None,
        };
        self.interface = interface.clone();
        let derives = match d {
            TypeDecl::Struct(s) => {
                self.generics(&mut s.generics)?;
                for f in &mut s.fields {
                    if let (Vis::Export(at), false) = (f.vis, d_vis.is_export()) {
                        return Err(not_exported_member("field", &f.name, &s.name, at));
                    }
                    // A field is seen no further than its struct.
                    let vis = match interface {
                        Some(_) if f.vis.level() > d_vis.level() => d_vis,
                        Some(_) => f.vis,
                        None => Vis::Private,
                    };
                    let field = format!("field `{}`", f.name.name);
                    self.interface = interface_of(vis, &field, "its type names");
                    self.ty(&mut f.ty)?;
                }
                self.interface = interface;
                &mut s.derives
            }
            TypeDecl::Enum(e) => {
                self.generics(&mut e.generics)?;
                for t in e.variants.iter_mut().flat_map(|v| &mut v.fields) {
                    self.ty(t)?;
                }
                &mut e.derives
            }
            TypeDecl::Alias(a) => {
                self.generics(&mut a.generics)?;
                // Whoever can name it can name what it stands for.
                if interface.is_some() {
                    let what = format!("`{}`", a.name.name);
                    self.interface = interface_of(d_vis, &what, "what it stands for names");
                }
                self.ty(&mut a.ty)?;
                self.interface = None;
                self.params.truncate(self.params.len() - n);
                return Ok(());
            }
        };
        // Each derived trait by its symbol: only core's three can be.
        for name in derives.iter_mut().flat_map(|d| &mut d.names) {
            let written = name.name.clone();
            let a_trait = written.contains("::")
                || matches!(
                    self.r.m.get(self.module, &written, Ns::Types),
                    Some(Binding {
                        item: Item::Trait(_),
                        ..
                    })
                );
            self.trait_ref(name)?;
            if ![DISCARD, COPY, EQ].contains(&name.name.as_str()) {
                let message = match !a_trait {
                    true => format!(
                        "can't derive `{written}`; only `Copy`, `Discard` and `Eq` can be derived"
                    ),
                    false => format!(
                        "only `core`'s `Discard`, `Copy` and `Eq` can be derived, and `{written}` \
                         here is `{}`",
                        match is_library_symbol(&name.name) {
                            true => name.name.clone(),
                            false => format!("crate::{}", name.name),
                        }
                    ),
                };
                return Err(Error::ty(name.span, message));
            }
        }
        for w in derives.iter_mut().flat_map(|d| &mut d.wheres).flatten() {
            for (_, items) in &mut w.bounds {
                for b in items {
                    self.bound(b)?;
                }
            }
        }
        self.interface = None;
        self.params.truncate(self.params.len() - n);
        Ok(())
    }

    fn generics(&mut self, g: &mut Generics) -> Result<(), Error> {
        for (_, items) in &mut g.bounds {
            for b in items {
                if !b.name.name.starts_with('\'') {
                    self.bound(b)?;
                }
            }
        }
        for (subject, items) in &mut g.wheres {
            self.ty(subject)?;
            for b in items {
                if !b.name.name.starts_with('\'') {
                    self.bound(b)?;
                }
            }
        }
        Ok(())
    }

    fn bound(&mut self, b: &mut BoundItem) -> Result<(), Error> {
        self.trait_ref(&mut b.name)?;
        for a in &mut b.assoc {
            if let AssocBoundKind::Bounds(traits) = &mut a.kind {
                for t in traits {
                    self.trait_ref(t)?;
                }
            }
        }
        for t in b.types_mut() {
            self.ty(t)?;
        }
        Ok(())
    }

    /// Function `f`, whose signature is part of an interface if
    /// `interface` says what to say of a type in it that isn't seen as far.
    fn function(
        &mut self,
        f: &mut FnDecl,
        interface: Option<(String, Level)>,
    ) -> Result<(), Error> {
        let n = f.generics.types.len();
        self.params
            .extend(f.generics.types.iter().map(|p| p.name.clone()));
        let outer = std::mem::replace(&mut self.interface, interface);
        self.generics(&mut f.generics)?;
        for p in &mut f.params {
            self.ty(&mut p.ty)?;
        }
        self.ty(&mut f.ret)?;
        for e in &mut f.ensures {
            self.pred(e)?;
        }
        self.interface = None;
        self.expr(&mut f.body)?;
        self.interface = outer;
        self.params.truncate(self.params.len() - n);
        Ok(())
    }

    fn ty(&mut self, t: &mut TypeExpr) -> Result<(), Error> {
        match &mut t.kind {
            TypeExprKind::Refined(_, base, pred) => {
                self.ty(base)?;
                self.pred(pred)?;
            }
            TypeExprKind::Named(name, args) => {
                for a in args.iter_mut() {
                    self.ty(a)?;
                }
                let name = Ident {
                    name: name.clone(),
                    span: t.span,
                    id: t.id,
                };
                self.type_path(&name)?;
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => {}
            TypeExprKind::Fn(params, ret, effects) => {
                for p in params {
                    self.ty(p)?;
                }
                self.ty(ret)?;
                for t in effects.iter_mut().flat_map(EffectSet::types_mut) {
                    self.ty(t)?;
                }
            }
            TypeExprKind::Ref(_, _, inner) | TypeExprKind::Slice(inner) => self.ty(inner)?,
            TypeExprKind::Array(inner, n) => {
                self.ty(inner)?;
                self.ty(n)?;
            }
            TypeExprKind::Nat(_) => {}
            TypeExprKind::Proj(..) => {
                // `a::Point`, read as a projection, if `a` is a module.
                if let Some(path) = plain_path(t)
                    && self.is_module(&first_segment(&path))?
                {
                    let b = self.path(&path, Ns::Types)?;
                    if !matches!(b.item, Item::Type(_)) {
                        return Err(Error::ty(
                            path.span,
                            format!("`{}` isn't a struct or enum", path.name),
                        ));
                    }
                    self.r.m.paths.insert(t.id, b.item);
                    t.kind = TypeExprKind::Named(path.name, Vec::new());
                    return Ok(());
                }
                let TypeExprKind::Proj(inner, tr, _) = &mut t.kind else {
                    unreachable!("matched above");
                };
                self.ty(inner)?;
                if let Some(tr) = tr {
                    self.trait_ref(tr)?;
                }
            }
            TypeExprKind::Impl(bounds) => {
                for b in bounds {
                    self.bound(b)?;
                }
            }
        }
        Ok(())
    }

    /// Predicate `e`, of a refinement or an `ensures`.
    fn pred(&mut self, e: &mut Expr) -> Result<(), Error> {
        let outer = std::mem::replace(&mut self.in_pred, true);
        let r = self.expr(e);
        self.in_pred = outer;
        r
    }

    fn expr(&mut self, e: &mut Expr) -> Result<(), Error> {
        if self.in_pred
            && let ExprKind::Call(callee, args) = &mut e.kind
            && let ExprKind::Var(name, targs) = &callee.kind
            && name == "old"
            && targs.is_empty()
            && args.len() == 1
        {
            return self.expr(&mut args[0]);
        }
        if let ExprKind::Block(stmts, tail) = &mut e.kind
            && let Some(i) = stmts
                .iter()
                .position(|s| matches!(s, Stmt::Let { els: Some(_), .. }))
        {
            self.let_else(stmts, tail, i, e.span);
        }
        match &mut e.kind {
            ExprKind::IfLet(..) => {
                let ExprKind::IfLet(c, then, els) = std::mem::replace(&mut e.kind, ExprKind::Unit)
                else {
                    unreachable!("matched above");
                };
                let of = ElseOf::IfLet(els.is_some());
                let els = els.map(|e| *e).unwrap_or_else(|| Expr {
                    kind: ExprKind::Unit,
                    span: Span::new(then.span.end, then.span.end),
                    id: self.id(),
                });
                e.kind = self.if_let(*c, *then, els, of);
                return self.expr(e);
            }
            ExprKind::WhileLet(label, _, body) => {
                no_break_value(body, "`while let`", label.as_ref().map(|l| &l.name.name), 0)?;
                let ExprKind::WhileLet(label, c, body) =
                    std::mem::replace(&mut e.kind, ExprKind::Unit)
                else {
                    unreachable!("matched above");
                };
                e.kind = self.while_let(label, *c, *body, e.span);
                return self.expr(e);
            }
            ExprKind::For(f) => {
                self.pattern(&mut f.pat)?;
                self.expr(&mut f.iter)?;
                self.expr(&mut f.body)?;
                no_break_value(&f.body, "`for`", f.label.as_ref().map(|l| &l.name.name), 0)?;
                let ExprKind::For(f) = std::mem::replace(&mut e.kind, ExprKind::Unit) else {
                    unreachable!("matched above");
                };
                e.kind = self.for_loop(*f, e.span);
                return Ok(());
            }
            ExprKind::Index(x, i, ..) => {
                self.expr(x)?;
                self.expr(i)?;
                let ExprKind::Index(x, i, brackets, written) =
                    std::mem::replace(&mut e.kind, ExprKind::Unit)
                else {
                    unreachable!("matched above");
                };
                // `*x.index(i)`, the method named where the `[` is, by no
                // text of its own.
                let method = Ident {
                    name: (if written { "index_mut" } else { "index" }).into(),
                    span: Span::new(brackets.start, brackets.start),
                    id: self.id(),
                };
                let call = Expr {
                    kind: ExprKind::MethodCall(x, method, Vec::new(), vec![*i]),
                    span: e.span,
                    id: self.id(),
                };
                e.kind = ExprKind::Unary(UnOp::Deref, Box::new(call));
                return Ok(());
            }
            ExprKind::Try(x, _) => {
                self.expr(x)?;
                let ExprKind::Try(x, q) = std::mem::replace(&mut e.kind, ExprKind::Unit) else {
                    unreachable!("matched above");
                };
                e.kind = self.try_expr(*x, q);
                return Ok(());
            }
            ExprKind::Range(l, r) => {
                self.expr(l)?;
                self.expr(r)?;
                if let (Some(a), Some(b)) = (constant(l), constant(r))
                    && a > b
                    && !self.r.m.in_library(self.module)
                {
                    self.r.m.warnings.push(Warning {
                        lint: Lint::EmptyRanges,
                        span: e.span,
                        message: format!(
                            "this range is empty: `{a}` is after `{b}`, so it never yields \
                             anything"
                        ),
                        fixes: Vec::new(),
                    });
                }
                let ExprKind::Range(l, r) = std::mem::replace(&mut e.kind, ExprKind::Unit) else {
                    unreachable!("matched above");
                };
                // `core::ops::Range::new(l, r)`, its type named where it
                // can't be by any other path: just after `l`.
                let ty = Ident {
                    name: "core::ops::Range".into(),
                    span: Span::new(l.span.end, l.span.end),
                    id: self.id(),
                };
                self.r.m.paths.insert(ty.id, self.core_item("ops", "Range"));
                let new = Ident {
                    name: "new".into(),
                    span: Span::new(r.span.start, r.span.start),
                    id: self.id(),
                };
                e.kind = ExprKind::Variant(ty, Vec::new(), new, vec![*l, *r], true);
                return Ok(());
            }
            ExprKind::Var(name, targs) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                if name.contains("::") {
                    let name = Ident {
                        name: name.clone(),
                        span: e.span,
                        id: e.id,
                    };
                    let b = self.path(&name, Ns::Values)?;
                    self.r.m.paths.insert(e.id, b.item);
                }
            }
            ExprKind::Variant(ty, targs, member, args, parens) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                for a in args.iter_mut() {
                    self.expr(a)?;
                }
                if self.is_module(ty)? {
                    // `a::f(1)`: a function in a module, not a variant.
                    if let Some(t) = targs.first() {
                        return Err(Error::ty(
                            t.span,
                            "a module has no type arguments; they go after the function",
                        ));
                    }
                    let path = Ident {
                        name: format!("{}::{}", ty.name, member.name),
                        span: ty.span.to(member.span),
                        id: self.id(),
                    };
                    let b = self.path(&path, Ns::Values)?;
                    let var = Expr {
                        kind: ExprKind::Var(path.name, Vec::new()),
                        span: path.span,
                        id: self.id(),
                    };
                    // Without `(..)`, it's this expression that's the path.
                    let named = if *parens { var.id } else { e.id };
                    self.r.m.paths.insert(named, b.item);
                    e.kind = match parens {
                        true => ExprKind::Call(Box::new(var), std::mem::take(args)),
                        false => var.kind,
                    };
                    return Ok(());
                }
                if ty.name.contains("::") {
                    let b = self.path(ty, Ns::Types)?;
                    match b.item {
                        Item::Type(_) => {
                            self.r.m.paths.insert(ty.id, b.item);
                        }
                        Item::Trait(k) => ty.name = self.symbols[k].clone(),
                        _ => {}
                    }
                } else if let Some(b) = self.r.m.get(self.module, &ty.name, Ns::Types)
                    && let Item::Trait(k) = b.item
                {
                    ty.name = self.symbols[k].clone();
                }
                return Ok(());
            }
            ExprKind::StructLit(ty, targs, _) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                self.type_path(&ty.clone())?;
            }
            ExprKind::MethodCall(_, _, targs, _) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
            }
            ExprKind::Block(stmts, _) => {
                for s in stmts.iter_mut() {
                    match s {
                        Stmt::Let { pat, ty, .. } => {
                            self.pattern(pat)?;
                            if let Some(t) = ty {
                                self.ty(t)?;
                            }
                        }
                        Stmt::Fn(f) => self.function(f, None)?,
                        Stmt::Type(d) => self.type_decl(d)?,
                        Stmt::Expr(_) | Stmt::BlockLike(_) => {}
                    }
                }
            }
            ExprKind::Match(_, arms) => {
                for a in arms.iter_mut() {
                    self.pattern(&mut a.pat)?;
                }
            }
            ExprKind::Lambda(l) => {
                let l = std::sync::Arc::make_mut(l);
                for p in &mut l.params {
                    if let Some(t) = &mut p.ty {
                        self.ty(t)?;
                    }
                }
                if let Some(t) = &mut l.ret {
                    self.ty(t)?;
                }
            }
            _ => {}
        }
        for c in children_mut(e) {
            self.expr(c)?;
        }
        Ok(())
    }

    /// Item `name` of the core library's module `module`, which it has.
    fn core_item(&self, module: &str, name: &str) -> Item {
        let core = self.r.m.core.expect("the core library is loaded");
        let Some(Binding {
            item: Item::Mod(m), ..
        }) = self.r.m.mods[core].types.get(module)
        else {
            unreachable!("the core library has a module `{module}`");
        };
        self.r.m.mods[*m].types[name].item
    }

    /// What `for pat in iter { body }`, `f`, at `span`, means (lemma C4):
    ///
    /// ```text
    /// {
    ///     let mut it = iter;
    ///     loop {
    ///         match core::iter::Iterator::next(&mut it) {
    ///             core::option::Option::Some(pat) => body,
    ///             core::option::Option::None => break,
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// with the `for`'s label on the `loop`, and `it` a name no program can
    /// write. So `it` is thrown away when the loop ends, which it can only
    /// be if it's `Discard`, and each value `pat` binds is the body's to use
    /// up, by the rules for any variable. Nothing about a `for` needs a rule
    /// of its own.
    ///
    /// Each node made here gets an id of its own, which is what the checker
    /// files what it learns about it by. Its span is only where messages
    /// and editors point: one in the `for` or `in`. The variable's is the
    /// iterator's, where an error about it should be.
    fn for_loop(&mut self, f: ForLoop, span: Span) -> ExprKind {
        let k = f.label.as_ref().map_or(span.start, |l| l.head.end);
        let i = f.in_span.start;
        let at = Span::new;
        let it = || ExprKind::Var(FOR_ITERATOR.into(), Vec::new());
        let iter_span = f.iter.span;
        let init = Stmt::Let {
            pat: Pattern {
                kind: PatternKind::Bind(FOR_ITERATOR.into(), true),
                span: iter_span,
                id: self.id(),
            },
            ty: None,
            init: Some(Box::new(f.iter)),
            els: None,
            span: at(k, iter_span.end),
        };
        let borrow = Expr {
            kind: ExprKind::Unary(
                UnOp::RefMut,
                Box::new(Expr {
                    kind: it(),
                    span: at(i + 1, i + 2),
                    id: self.id(),
                }),
            ),
            span: at(i, i + 1),
            id: self.id(),
        };
        let iterator = &self.symbols[match self.core_item("iter", "Iterator") {
            Item::Trait(k) => k,
            _ => unreachable!("`core::iter::Iterator` is a trait"),
        }];
        let next = Expr {
            kind: ExprKind::Variant(
                Ident {
                    name: iterator.clone(),
                    span: at(k + 2, k + 2),
                    id: self.id(),
                },
                Vec::new(),
                // Where an error about it is: `x` isn't an iterator.
                Ident {
                    name: "next".into(),
                    span: at(iter_span.start, iter_span.start),
                    id: self.id(),
                },
                vec![borrow],
                true,
            ),
            span: at(k, iter_span.end),
            id: self.id(),
        };
        let option = self.core_item("option", "Option");
        let mut variant = |name: &str, pats: Vec<Pattern>, ty: Span, member: Span, span: Span| {
            let ty_id = self.id();
            self.r.m.paths.insert(ty_id, option);
            Pattern {
                kind: PatternKind::Variant(
                    Ident {
                        name: "core::option::Option".into(),
                        span: ty,
                        id: ty_id,
                    },
                    Vec::new(),
                    Ident {
                        name: name.into(),
                        span: member,
                        id: self.id(),
                    },
                    pats,
                ),
                span,
                id: self.id(),
            }
        };
        let some = variant(
            "Some",
            vec![f.pat],
            at(k, k),
            at(k + 3, k + 3),
            at(k, k + 1),
        );
        let none = variant(
            "None",
            Vec::new(),
            at(k + 1, k + 1),
            at(i, i),
            at(k + 1, k + 2),
        );
        let arms = vec![
            Arm {
                pat: some,
                body: f.body,
            },
            Arm {
                pat: none,
                body: Expr {
                    kind: ExprKind::Break(None, None),
                    span: at(k + 2, k + 2),
                    id: self.id(),
                },
            },
        ];
        let step = Expr {
            kind: ExprKind::Match(Box::new(next), arms),
            span: at(k, k + 2),
            id: self.id(),
        };
        let body = Expr {
            kind: ExprKind::Block(vec![Stmt::BlockLike(step)], None),
            span: at(k + 1, k + 2),
            id: self.id(),
        };
        let looped = Expr {
            kind: ExprKind::Loop(f.label, Box::new(body)),
            span: at(k, k + 3),
            id: self.id(),
        };
        ExprKind::Block(vec![init, Stmt::BlockLike(looped)], None)
    }

    /// What `x?`, with its `?` at `q`, means (lemma Q1):
    ///
    /// ```text
    /// match core::ops::Try::branch(x) {
    ///     core::ops::ControlFlow::Continue(v) => v,
    ///     core::ops::ControlFlow::Break(r) => return core::ops::Try::from_residual(r),
    /// }
    /// ```
    ///
    /// with `v` and `r` a name no program can write, and the library's items
    /// named by their symbols, so a program's own `Try` doesn't change it.
    /// The calls are at the `?`, which is where an error about them is: what
    /// `x` is (`branch`), or what the function returns (`from_residual`).
    fn try_expr(&mut self, x: Expr, q: Span) -> ExprKind {
        // The names it's made of are at an empty span, at the `?`: where
        // errors about them are, and as names the program doesn't have, with
        // nothing to show or rename (`info::unwritten`).
        let at = Span::new(q.start, q.start);
        self.r.m.tries.insert(at);
        let try_trait = &self.symbols[match self.core_item("ops", "Try") {
            Item::Trait(k) => k,
            _ => unreachable!("`core::ops::Try` is a trait"),
        }]
        .clone();
        let call = |cx: &mut Self, method: &str, arg: Expr| {
            let name = Ident {
                name: method.into(),
                span: at,
                id: cx.id(),
            };
            Expr {
                kind: ExprKind::Variant(
                    Ident {
                        name: try_trait.clone(),
                        span: at,
                        id: cx.id(),
                    },
                    Vec::new(),
                    name,
                    vec![arg],
                    true,
                ),
                span: q,
                id: cx.id(),
            }
        };
        let var = |cx: &mut Self| Expr {
            kind: ExprKind::Var(TRY_VALUE.into(), Vec::new()),
            span: at,
            id: cx.id(),
        };
        let branch = call(self, "branch", x);
        let control_flow = self.core_item("ops", "ControlFlow");
        let arm = |cx: &mut Self, variant: &str, body: Expr| {
            let ty_id = cx.id();
            cx.r.m.paths.insert(ty_id, control_flow);
            let bind = Pattern {
                kind: PatternKind::Bind(TRY_VALUE.into(), false),
                span: at,
                id: cx.id(),
            };
            let pat = Pattern {
                kind: PatternKind::Variant(
                    Ident {
                        name: "core::ops::ControlFlow".into(),
                        span: at,
                        id: ty_id,
                    },
                    Vec::new(),
                    Ident {
                        name: variant.into(),
                        span: at,
                        id: cx.id(),
                    },
                    vec![bind],
                ),
                span: q,
                id: cx.id(),
            };
            Arm { pat, body }
        };
        let value = var(self);
        let go_on = arm(self, "Continue", value);
        let residual = var(self);
        let from_residual = call(self, "from_residual", residual);
        let stop = Expr {
            kind: ExprKind::Return(Some(Box::new(from_residual))),
            span: q,
            id: self.id(),
        };
        let stop = arm(self, "Break", stop);
        ExprKind::Match(Box::new(branch), vec![go_on, stop])
    }

    /// What `if let pat = x { then } else { els }` means (lemma L1 of
    /// step 28), and `while let` and `let .. else` too, as `of` says:
    ///
    /// ```text
    /// match x {
    ///     pat => then,
    ///     other => els,
    /// }
    /// ```
    ///
    /// where `other` matches what `pat` doesn't (`Rewrite::else_pattern`).
    /// Nothing about them needs a rule of its own. The made-up pattern is
    /// where the `let` is, which is where errors about it are, in terms of
    /// what was written (`Modules::elses`).
    fn if_let(&mut self, c: LetCond, then: Expr, els: Expr, of: ElseOf) -> ExprKind {
        let at = c.let_span;
        self.r.m.elses.entry(at).or_insert(of);
        let other = self.else_pattern(&c.pat, at);
        ExprKind::Match(
            Box::new(c.expr),
            vec![
                Arm {
                    pat: c.pat,
                    body: then,
                },
                Arm {
                    pat: other,
                    body: els,
                },
            ],
        )
    }

    /// What `'a: while let pat = x { body }`, at `span`, means:
    ///
    /// ```text
    /// 'a: loop {
    ///     match x {
    ///         pat => body,
    ///         other => break,
    ///     }
    /// }
    /// ```
    ///
    /// with `other` as in `Rewrite::if_let`. Its body can't `break` with a
    /// value, as a `while`'s can't, which the caller checks.
    fn while_let(&mut self, label: Option<Label>, c: LetCond, body: Expr, span: Span) -> ExprKind {
        let at = Span::new(c.let_span.start, c.let_span.start);
        let stop = Expr {
            kind: ExprKind::Break(None, None),
            span: at,
            id: self.id(),
        };
        let step = Expr {
            kind: self.if_let(c, body, stop, ElseOf::WhileLet),
            span,
            id: self.id(),
        };
        let body = Expr {
            kind: ExprKind::Block(vec![Stmt::BlockLike(step)], None),
            span,
            id: self.id(),
        };
        ExprKind::Loop(label, Box::new(body))
    }

    /// Rewrites the `let .. else` that's statement `i` of a block, at
    /// `span`, with the rest of the block, as what they mean (lemma L2 of
    /// step 28):
    ///
    /// ```text
    /// { before; let pat = x else { els }; after }
    /// ```
    ///
    /// is
    ///
    /// ```text
    /// {
    ///     before;
    ///     match x {
    ///         pat => { after },
    ///         other => { let _: ! = { els }; },
    ///     }
    /// }
    /// ```
    ///
    /// So what `pat` binds is there in `after` and not in `els`, and `els`
    /// can't finish, as it's a `!`. The functions and types declared in
    /// `after` stay in the outer block, all of which they're in scope in;
    /// they can't use the locals around them, so where in it they are
    /// makes no difference to them.
    fn let_else(
        &mut self,
        stmts: &mut Vec<Stmt>,
        tail: &mut Option<Box<Expr>>,
        i: usize,
        span: Span,
    ) {
        let rest = stmts.split_off(i + 1);
        let Some(Stmt::Let {
            pat,
            init: Some(init),
            els: Some(els),
            span: at,
            ..
        }) = stmts.pop()
        else {
            unreachable!("a `let .. else`, which the parser gives a value");
        };
        let (items, after): (Vec<Stmt>, Vec<Stmt>) = rest
            .into_iter()
            .partition(|s| matches!(s, Stmt::Fn(_) | Stmt::Type(_)));
        stmts.extend(items);
        let let_span = Span::new(at.start, at.start + 3);
        self.r.m.elses.insert(let_span, ElseOf::LetElse);
        let after = Expr {
            kind: ExprKind::Block(after, tail.take()),
            span: Span::new(at.end, span.end),
            id: self.id(),
        };
        let never = Pattern {
            kind: PatternKind::Wild,
            span: Span::new(els.span.start, els.span.start),
            id: self.id(),
        };
        self.r.m.must_leave.insert(never.id);
        let leave = Stmt::Let {
            pat: never,
            ty: Some(Box::new(TypeExpr {
                kind: TypeExprKind::Never,
                span: Span::new(els.span.start, els.span.start),
                id: self.id(),
            })),
            span: els.span,
            init: Some(els),
            els: None,
        };
        let els = Expr {
            kind: ExprKind::Block(vec![leave], None),
            span: Span::new(at.end, at.end),
            id: self.id(),
        };
        let c = LetCond {
            let_span,
            pat,
            expr: *init,
        };
        *tail = Some(Box::new(Expr {
            kind: ExprKind::IfLet(Box::new(c), Box::new(after), Some(Box::new(els))),
            span: Span::new(at.start, span.end),
            id: self.id(),
        }));
    }

    /// A pattern that matches what `p` doesn't, and takes it apart as far
    /// as `p` looks at it (lemma L3 of step 28): the variants of its enum
    /// that `p` doesn't match whatever is in them, with a `_` for each
    /// field, its struct with `..`, or an array of `_`. So it uses up what
    /// it matches, as any pattern of a struct or variant does, and throws
    /// away only what's in it, which no code can name, so that must be
    /// `Discard`. A `_` where it can't tell which enum `p` is of, by its
    /// name at the level of the module (a local type's, or an alias's):
    /// that leaves a linear value unused, which is an error, so it only
    /// rejects more. Matching all `p` doesn't is all it's for: if it's
    /// wrong, the `match` isn't exhaustive, which is an error too.
    ///
    /// Its alternatives all have one span, the `let`'s, which is how the
    /// exhaustiveness checker tells them apart: so one that's never
    /// reached, as `Some(_)` after `Some(0..=255)` of a `u8`, isn't
    /// reported, only all of them, as the arm, when `p` always matches.
    fn else_pattern(&mut self, p: &Pattern, at: Span) -> Pattern {
        let empty = Span::new(at.start, at.start);
        let alts: Vec<&Pattern> = match &p.kind {
            PatternKind::Or(alts) => alts.iter().collect(),
            _ => vec![p],
        };
        let mut kind = PatternKind::Wild;
        match &alts[0].kind {
            PatternKind::Variant(ty, ..)
                if alts.iter().all(
                    |a| matches!(&a.kind, PatternKind::Variant(t, ..) if t.name == ty.name),
                ) =>
            {
                if let Some(variants) = self.variants_of(ty) {
                    let missed: Vec<(String, usize)> = variants
                        .into_iter()
                        .filter(|(name, _)| {
                            !alts.iter().any(|a| match &a.kind {
                                PatternKind::Variant(_, _, v, pats) => {
                                    v.name == *name && !pats.iter().any(|p| self.refutable(p))
                                }
                                _ => false,
                            })
                        })
                        .collect();
                    let mut made: Vec<Pattern> = Vec::new();
                    for (name, n) in missed {
                        let fields = (0..n).map(|_| self.wild(at)).collect();
                        let kind = PatternKind::Variant(
                            Ident {
                                name: ty.name.clone(),
                                span: empty,
                                id: self.id(),
                            },
                            Vec::new(),
                            Ident {
                                name,
                                span: empty,
                                id: self.id(),
                            },
                            fields,
                        );
                        made.push(Pattern {
                            kind,
                            span: at,
                            id: self.id(),
                        });
                    }
                    kind = match made.len() {
                        0 => PatternKind::Wild,
                        1 => made.pop().expect("one").kind,
                        _ => PatternKind::Or(made),
                    };
                }
            }
            PatternKind::Struct(ty, ..) if alts.len() == 1 && self.refutable(p) => {
                kind = PatternKind::Struct(
                    Ident {
                        name: ty.name.clone(),
                        span: empty,
                        id: self.id(),
                    },
                    Vec::new(),
                    Vec::new(),
                    true,
                );
            }
            PatternKind::Array(pats) if alts.len() == 1 && self.refutable(p) => {
                kind = PatternKind::Array(pats.iter().map(|_| self.wild(at)).collect());
            }
            _ => {}
        }
        Pattern {
            kind,
            span: at,
            id: self.id(),
        }
    }

    fn wild(&mut self, at: Span) -> Pattern {
        Pattern {
            kind: PatternKind::Wild,
            span: at,
            id: self.id(),
        }
    }

    /// The variants of the enum `ty` names, as far as the module can tell:
    /// not a local type's, nor an alias's.
    fn variants_of(&mut self, ty: &Ident) -> Option<Vec<(String, usize)>> {
        let k = if ty.name.contains("::") {
            match self.path(ty, Ns::Types).ok()?.item {
                Item::Type(k) => k,
                _ => return None,
            }
        } else if self.params.contains(&ty.name) {
            return None;
        } else {
            self.r.m.type_named(self.module, &ty.name)?
        };
        self.variants.get(k)?.clone()
    }

    /// Can `p` fail to match, as far as its shape says? Where it can't
    /// tell, yes: then the `else` an `if let` with it means has an arm that
    /// may never match, which only adds what it throws away.
    fn refutable(&mut self, p: &Pattern) -> bool {
        match &p.kind {
            PatternKind::Wild | PatternKind::Bind(..) | PatternKind::Unit => false,
            PatternKind::Int(..) | PatternKind::Range(..) | PatternKind::Bool(_) => true,
            PatternKind::Variant(ty, _, _, pats) => {
                self.variants_of(ty).is_none_or(|vs| vs.len() != 1)
                    || pats.iter().any(|p| self.refutable(p))
            }
            PatternKind::Struct(_, _, fields, _) => fields.iter().any(|f| self.refutable(&f.pat)),
            PatternKind::Array(pats) => pats.iter().any(|p| self.refutable(p)),
            PatternKind::Or(alts) => alts.iter().all(|p| self.refutable(p)),
        }
    }

    fn pattern(&mut self, p: &mut Pattern) -> Result<(), Error> {
        match &mut p.kind {
            PatternKind::Variant(ty, targs, _, pats) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                self.type_path(&ty.clone())?;
                for p in pats {
                    self.pattern(p)?;
                }
            }
            PatternKind::Struct(ty, targs, fields, _) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                self.type_path(&ty.clone())?;
                for f in fields {
                    self.pattern(&mut f.pat)?;
                }
            }
            PatternKind::Or(alts) | PatternKind::Array(alts) => {
                for p in alts {
                    self.pattern(p)?;
                }
            }
            PatternKind::Wild
            | PatternKind::Bind(..)
            | PatternKind::Int(..)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
        Ok(())
    }
}

/// The name of the variable a `for` loop keeps its iterator in, which no
/// program can write (see `Rewrite::for_loop`).
pub(crate) const FOR_ITERATOR: &str = "{for}";

/// The name of the variable each arm of what `?` means binds, which no
/// program can write (see `Rewrite::try_expr`).
pub(crate) const TRY_VALUE: &str = "{try}";

/// Checks that no `break` in `e`, the body of a `for` or `while let`
/// (`what`) labelled `label`, if it is, gives that loop a value: it can end
/// without a `break`, so it can only be `()`. `depth` is how many loops in
/// it `e` is, which an unlabelled `break` would be for instead.
fn no_break_value(e: &Expr, what: &str, label: Option<&String>, depth: usize) -> Result<(), Error> {
    match &e.kind {
        ExprKind::Break(l, Some(_)) => {
            let ours = match l {
                Some(l) => Some(&l.name) == label,
                None => depth == 0,
            };
            if ours {
                return Err(Error::ty(
                    e.span,
                    format!(
                        "a `break` with a value only works in a `loop`: a {what} loop can end \
                         without a `break`, and then it has no value to give"
                    ),
                ));
            }
        }
        // A closure's body is a function of its own.
        ExprKind::Lambda(_) => return Ok(()),
        ExprKind::Loop(..) | ExprKind::While(..) | ExprKind::WhileLet(..) | ExprKind::For(_) => {
            for c in e.children() {
                no_break_value(c, what, label, depth + 1)?;
            }
            return Ok(());
        }
        _ => {}
    }
    for c in e.children() {
        no_break_value(c, what, label, depth)?;
    }
    Ok(())
}

/// The value of `e`, if it's an integer constant: `3` or `-3`.
fn constant(e: &Expr) -> Option<i128> {
    match &e.kind {
        ExprKind::Int(n, _) => Some(i128::from(*n)),
        ExprKind::Unary(UnOp::Neg, x) => constant(x)?.checked_neg(),
        _ => None,
    }
}

/// The segments of path `name`, as the parser joined them (`Parser::path`):
/// the first starts where it does, and the last ends where it does. Those
/// between get the span of the whole, as where each is isn't kept.
fn segments(name: &Ident) -> Vec<Ident> {
    let parts: Vec<&str> = name.name.split("::").collect();
    let n = parts.len();
    parts
        .into_iter()
        .enumerate()
        .map(|(i, part)| {
            let span = match i {
                0 => Span::new(name.span.start, name.span.start + part.len()),
                i if i + 1 == n => Span::new(name.span.end - part.len(), name.span.end),
                _ => name.span,
            };
            Ident {
                name: part.to_string(),
                span,
                id: name.id,
            }
        })
        .collect()
}

/// The first segment of path `name`.
fn first_segment(name: &Ident) -> Ident {
    segments(name).swap_remove(0)
}

/// `a::b::C`, if `t` is a chain of projections without traits read from
/// one: as one name, spanning it.
fn plain_path(t: &TypeExpr) -> Option<Ident> {
    match &t.kind {
        TypeExprKind::Named(name, args) if args.is_empty() => Some(Ident {
            name: name.clone(),
            span: t.span,
            id: t.id,
        }),
        TypeExprKind::Proj(inner, None, name) => {
            let inner = plain_path(inner)?;
            Some(Ident {
                name: format!("{}::{}", inner.name, name.name),
                span: inner.span.to(name.span),
                id: t.id,
            })
        }
        _ => None,
    }
}

/// The expressions directly in `e`, to change, as `Expr::children` gives
/// them.
pub(crate) fn children_mut(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Int(..)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::ByteStr(_)
        | ExprKind::Unit
        | ExprKind::Var(..)
        | ExprKind::Continue(_) => Vec::new(),
        ExprKind::Unary(_, x)
        | ExprKind::As(x, _)
        | ExprKind::Field(x, _)
        | ExprKind::Loop(_, x)
        | ExprKind::Unsafe(x) => vec![&mut **x],
        ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter_mut().map(|x| &mut **x).collect(),
        ExprKind::Binary(_, l, r)
        | ExprKind::Assign(_, l, r)
        | ExprKind::While(_, l, r)
        | ExprKind::Range(l, r)
        | ExprKind::Index(l, r, ..) => {
            vec![&mut **l, &mut **r]
        }
        ExprKind::For(f) => vec![&mut f.iter, &mut f.body],
        ExprKind::IfLet(c, then, els) => [&mut c.expr, &mut **then]
            .into_iter()
            .chain(els.as_deref_mut())
            .collect(),
        ExprKind::WhileLet(_, c, body) => vec![&mut c.expr, &mut **body],
        ExprKind::Try(e, _) => vec![e],
        ExprKind::Call(f, args) | ExprKind::MethodCall(f, _, _, args) => {
            std::iter::once(&mut **f).chain(args.iter_mut()).collect()
        }
        ExprKind::If(c, then, els) => [&mut **c, &mut **then]
            .into_iter()
            .chain(els.as_deref_mut())
            .collect(),
        ExprKind::Block(stmts, tail) => stmts
            .iter_mut()
            .flat_map(|s| match s {
                Stmt::Let { init, els, .. } => init
                    .as_deref_mut()
                    .into_iter()
                    .chain(els.as_deref_mut())
                    .collect(),
                Stmt::Expr(e) | Stmt::BlockLike(e) => vec![e],
                Stmt::Fn(_) | Stmt::Type(_) => Vec::new(),
            })
            .chain(tail.as_deref_mut())
            .collect(),
        ExprKind::StructLit(_, _, inits) => inits.iter_mut().map(|i| &mut i.value).collect(),
        ExprKind::Variant(_, _, _, args, ..) | ExprKind::Array(args) => args.iter_mut().collect(),
        ExprKind::Repeat(v, n) => vec![&mut **v, &mut **n],
        ExprKind::Match(scrutinee, arms) => std::iter::once(&mut **scrutinee)
            .chain(arms.iter_mut().map(|a| &mut a.body))
            .collect(),
        ExprKind::Lambda(l) => vec![&mut std::sync::Arc::make_mut(l).body],
    }
}

/// What to say of a type in the signature of `f` that isn't seen as far as
/// `f`, if that's part of an interface: if it's `pub` or `export`, and, for
/// a method, `inherent`.
fn signature(f: &FnDecl, inherent: bool) -> Option<(String, Level)> {
    let what = format!("`{}`", f.name.name);
    inherent
        .then(|| interface_of(f.vis, &what, "its signature names"))
        .flatten()
}

/// The interface of `what`, which is seen as `vis` says, if it's `pub` or
/// `export`: what to say of a type in it that isn't seen as far, "`f` is
/// exported, but its signature names", and how far it must be seen.
fn interface_of(vis: Vis, what: &str, names: &str) -> Option<(String, Level)> {
    let is = match vis {
        Vis::Private => return None,
        Vis::Pub(_) => "public",
        Vis::Export(_) => "exported",
    };
    Some((format!("{what} is {is}, but {names}"), vis.level()))
}

/// Is `symbol` an item of a library built into the compiler: does it start
/// with one's name, as `alloc::boxed::Box` does?
pub(crate) fn is_library_symbol(symbol: &str) -> bool {
    let first = symbol.split("::").next().unwrap_or_default();
    crate::load::LIBRARIES
        .iter()
        .any(|(name, ..)| *name == first)
}
