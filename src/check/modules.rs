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
//! in that, and, if it's `pub`, wherever its module is. A path is resolved
//! segment by segment, each of which must be visible where the path is
//! written. A `use` binds a name like an item declared there, private
//! unless it's `pub use`, and can't make public what isn't.
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
use crate::error::{Error, Lint, Span, Warning};
use std::cell::Cell;
use std::collections::HashMap;

/// An item, by where it is in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Item {
    /// By its index in `Program::mods`.
    Mod(usize),
    /// By its index in `Program::types`.
    Type(usize),
    /// By its index in `Program::traits`.
    Trait(usize),
    /// By its index in `Program::fns`, which is also its `FnId`.
    Fn(usize),
}

/// A name in a module's namespace: an item declared there, or brought in
/// by a `use`.
#[derive(Debug, Clone)]
pub(crate) struct Binding {
    pub(crate) item: Item,
    /// The module it's declared or imported in.
    pub(crate) module: usize,
    /// Is it `pub` there?
    pub(crate) public: bool,
    /// The `use` it's from, by its index in `Modules::imports`, if it is.
    pub(crate) import: Option<usize>,
}

#[derive(Debug, Default)]
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

/// A name a `use` brings in, and whether anything uses it.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) name: Ident,
    /// The whole `use`, for a fix that removes it, if it brings in only
    /// this.
    pub(crate) alone: Option<Span>,
    /// Is it a `pub use`, which other modules may use?
    pub(crate) public: bool,
    pub(crate) used: Cell<bool>,
}

#[derive(Debug, Default)]
pub(crate) struct Modules {
    pub(crate) mods: Vec<Module>,
    /// What each path with a `::` refers to, by its node: one to a type
    /// where a type is written (`a::Point`, by the type), in a struct
    /// literal or pattern (by its name), and one to a function where one is
    /// used (by the expression).
    pub(crate) paths: HashMap<NodeId, Item>,
    pub(crate) imports: Vec<Import>,
    /// The symbol of each trait, by its index in `Program::traits`.
    pub(crate) trait_symbols: Vec<String>,
    /// Where each module ends: before its `}`, or at the end of the file.
    pub(crate) ends: Vec<usize>,
    /// The last segment of the path of each name a `use` brings in, and
    /// what it names: something in each namespace.
    pub(crate) leaf_items: Vec<(Ident, Vec<Item>)>,
    /// The core library's root module, and its prelude.
    pub(crate) core: Option<usize>,
    /// The alloc library's root module.
    pub(crate) alloc: Option<usize>,
    pub(crate) prelude: Option<usize>,
    /// Warnings about what's rewritten here: empty ranges.
    pub(crate) warnings: Vec<Warning>,
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
    /// there? The built-in ones are everywhere.
    pub(crate) fn trait_in_scope(&self, m: usize, symbol: &str) -> bool {
        let builtin = crate::traits::BUILTIN_TRAITS.contains(&symbol)
            || crate::traits::FN_TRAITS.contains(&symbol);
        builtin || self.trait_binding(m, symbol).is_some()
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
            _ if symbol.starts_with("core::") || symbol.starts_with("alloc::") => {
                symbol.to_string()
            }
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
                    .is_some_and(|b| b.public && b.import.is_none())
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
        let root = Some(self.package_root(m));
        root == self.core || root == self.alloc
    }

    /// The root module of the library `name`, if it's one built in, as a
    /// path can start with: `core` or `alloc`.
    pub(crate) fn library(&self, name: &str) -> Option<usize> {
        match name {
            "core" => self.core,
            "alloc" => self.alloc,
            _ => None,
        }
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
    /// `module`, `pub` (`public`) or not?
    pub(crate) fn visible(&self, module: usize, public: bool, from: usize) -> bool {
        public || self.within(from, module)
    }

    /// `m` for messages: "`a::b`", or "the crate root".
    pub(crate) fn describe(&self, m: usize) -> String {
        match m {
            ROOT => "the crate root".into(),
            m => format!("`{}`", self.mods[m].path),
        }
    }
}

/// The symbol of trait `t`: its name, after its module's path if it's not
/// at the crate root.
fn trait_symbol(mods: &[Module], t: &TraitDecl) -> String {
    format!("{}{}", mods[t.module].prefix, t.name.name)
}

/// Resolves the paths of `prog`, rewriting it as the module docs say.
pub(crate) fn resolve(prog: &mut Program) -> Result<Modules, Error> {
    let mut mods: Vec<Module> = Vec::new();
    for (i, m) in prog.mods.iter().enumerate() {
        let (prefix, path) = match m.parent {
            None => (String::new(), String::new()),
            Some(p) => {
                let path = match p {
                    ROOT => m.name.name.clone(),
                    p => format!("{}::{}", mods[p].path, m.name.name),
                };
                (format!("{path}::"), path)
            }
        };
        debug_assert_eq!(i, mods.len());
        // A library's root is a package's: no module is around it.
        let parent = match prog.is_library_root(i) {
            true => None,
            false => m.parent,
        };
        mods.push(Module {
            parent,
            prefix,
            path,
            ..Module::default()
        });
    }
    let mut r = Resolver {
        m: Modules {
            mods,
            core: prog.core,
            alloc: prog.alloc,
            ..Modules::default()
        },
        leaves: Vec::new(),
        states: Vec::new(),
        decls: HashMap::new(),
    };
    r.declare(prog)?;
    r.imports(prog)?;
    r.m.prelude = prog
        .core
        .and_then(|c| match r.m.mods[c].types.get("prelude") {
            Some(Binding {
                item: Item::Mod(p), ..
            }) => Some(*p),
            _ => None,
        });
    let symbols: Vec<String> = prog
        .traits
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
    r.m.trait_symbols = symbols.clone();
    let mut w = Rewrite {
        r: &mut r,
        symbols: &symbols,
        module: ROOT,
        params: Vec::new(),
        interface: None,
        next_id: prog.next_id,
    };
    let rewritten = w.program(prog);
    // What's rewritten stays rewritten, ids and all, even past an error.
    prog.next_id = w.next_id;
    rewritten?;
    for (t, symbol) in prog.traits.iter_mut().zip(symbols) {
        t.name.name = symbol;
    }
    Ok(r.m)
}

/// A name a `use` brings in, as written.
struct Leaf {
    module: usize,
    path: Vec<Ident>,
    name: Ident,
    public: Option<Span>,
    decl: Span,
}

/// How far resolving a `use` has got.
#[derive(Clone)]
enum State {
    Todo,
    Doing,
    /// What its path names, in each namespace.
    Done(Option<Binding>, Option<Binding>),
}

struct Resolver {
    m: Modules,
    /// Where each item is declared, and whether it's `pub` there.
    decls: HashMap<Item, (usize, bool)>,
    leaves: Vec<Leaf>,
    states: Vec<State>,
}

impl Resolver {
    /// Puts each item in its module's namespace.
    fn declare(&mut self, prog: &Program) -> Result<(), Error> {
        let mut items: Vec<(usize, &Ident, bool, Item, Ns)> = Vec::new();
        for (i, m) in prog.mods.iter().enumerate().skip(1) {
            // A library isn't a module of the crate root's.
            if prog.is_library_root(i) {
                continue;
            }
            let parent = m.parent.expect("not the crate root");
            items.push((parent, &m.name, m.public.is_some(), Item::Mod(i), Ns::Types));
        }
        for (k, d) in prog.types.iter().enumerate() {
            let public = d.public().is_some();
            items.push((d.module(), d.name(), public, Item::Type(k), Ns::Types));
        }
        for (k, t) in prog.traits.iter().enumerate() {
            let public = t.public.is_some();
            items.push((t.module, &t.name, public, Item::Trait(k), Ns::Types));
        }
        for (i, f) in prog.fns.iter().enumerate() {
            let public = f.public.is_some();
            items.push((f.module, &f.name, public, Item::Fn(i), Ns::Values));
        }
        items.sort_by_key(|(_, name, ..)| name.span.start);
        for &(module, _, public, item, _) in &items {
            self.decls.insert(item, (module, public));
        }
        for (module, name, public, item, ns) in items {
            let binding = Binding {
                item,
                module,
                public,
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
    fn imports(&mut self, prog: &Program) -> Result<(), Error> {
        for u in &prog.uses {
            for (path, name) in u.tree.leaves() {
                self.leaves.push(Leaf {
                    module: u.module,
                    path: path.into_iter().cloned().collect(),
                    name: name.clone(),
                    public: u.public,
                    decl: u.span,
                });
            }
        }
        self.states = vec![State::Todo; self.leaves.len()];
        for i in 0..self.leaves.len() {
            self.leaf(i)?;
        }
        for (i, leaf) in self.leaves.iter().enumerate() {
            let alone = self.leaves.iter().filter(|l| l.decl == leaf.decl).count() == 1;
            self.m.imports.push(Import {
                name: leaf.name.clone(),
                alone: alone.then_some(leaf.decl),
                public: leaf.public.is_some(),
                used: Cell::new(false),
            });
            let State::Done(types, values) = &self.states[i] else {
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
                    public: leaf.public.is_some(),
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
        match &self.states[i] {
            State::Done(t, v) => return Ok((t.clone(), v.clone())),
            State::Doing => {
                return Err(Error::ty(
                    self.leaves[i].decl,
                    "these imports go round in a circle: each needs another to say what it is",
                ));
            }
            State::Todo => {}
        }
        self.states[i] = State::Doing;
        let leaf = &self.leaves[i];
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
                .is_some_and(|b| self.m.visible(b.module, b.public, from))
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
        let leaf = &self.leaves[i];
        if leaf.public.is_some() && [&types, &values].into_iter().flatten().any(|b| !b.public) {
            return Err(Error::ty(
                last.span,
                format!(
                    "`{}` is private, so `pub use` can't make it public",
                    last.name
                ),
            ));
        }
        self.states[i] = State::Done(types.clone(), values.clone());
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
        let leaves: Vec<usize> = (0..self.leaves.len())
            .filter(|&i| self.leaves[i].module == m && self.leaves[i].name.name == name)
            .collect();
        for i in leaves {
            let (types, values) = self.leaf(i)?;
            let target = match ns {
                Ns::Types => types,
                Ns::Values => values,
            };
            if let Some(t) = target {
                let public = self.leaves[i].public.is_some();
                return Ok(Some(Binding {
                    item: t.item,
                    module: m,
                    public,
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
                        && let Some(library) = self.m.library(name)
                    {
                        m = library;
                        continue;
                    }
                    let Some(b) = found else {
                        return Err(match i {
                            0 => Error::ty(seg.span, format!("unknown module `{name}`")),
                            _ => no_item(&self.m, m, seg),
                        });
                    };
                    if i > 0 && !self.m.visible(b.module, b.public, from) {
                        return Err(private(&self.m, name, &b, seg.span));
                    }
                    match b.item {
                        Item::Mod(inner) => inner,
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
    Error::ty(
        at,
        format!("{what} is private to {}", mods.describe(b.module)),
    )
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
    /// While in the interface of a public item, what to say of a private
    /// type or trait in it: "`f` is public, but its signature names".
    interface: Option<String>,
    /// `Program::next_id`, while the program is being rewritten: the nodes
    /// made here take ids from it.
    next_id: u32,
}

impl Rewrite<'_> {
    /// An id for a node made here (see `NodeId`).
    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    fn program(&mut self, prog: &mut Program) -> Result<(), Error> {
        for d in &mut prog.types {
            self.module = d.module();
            self.type_decl(d)?;
        }
        for t in &mut prog.traits {
            self.module = t.module;
            self.params.push("Self".into());
            let interface = t
                .public
                .map(|_| format!("trait `{}` is public, but it names", t.name.name));
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
        for d in &mut prog.impls {
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
            for f in &mut d.methods {
                let interface = signature(f, d.trait_name.is_none());
                self.function(f, interface)?;
            }
            self.params.truncate(self.params.len() - params);
        }
        for f in &mut prog.fns {
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
        if !self.r.m.visible(b.module, b.public, from) {
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
            if self.r.m.library(first).is_none() {
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
    /// interface being walked, if it's a public item's: that it's public
    /// too (lemma M5 of the docs). Otherwise code that can use the item
    /// couldn't write down what it takes or gives.
    fn exposed(&self, item: Item, name: &str, at: Span) -> Result<(), Error> {
        let Some(interface) = &self.interface else {
            return Ok(());
        };
        let (module, public) = self.r.decls[&item];
        if public {
            return Ok(());
        }
        let name = name.rsplit("::").next().unwrap_or(name);
        Err(Error::ty(
            at,
            format!(
                "{interface} `{name}`, which is private to {}",
                self.r.m.describe(module)
            ),
        ))
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
            // A type parameter, or `int`, or a local type, or one of the
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
        let public = d.public().is_some() && self.interface.is_none();
        let interface = public.then(|| format!("`{}` is public, but it names", d.name().name));
        self.interface = interface.clone();
        let derives = match d {
            TypeDecl::Struct(s) => {
                self.generics(&mut s.generics)?;
                for f in &mut s.fields {
                    self.interface = match public && f.public.is_some() {
                        true => Some(format!(
                            "field `{}` is public, but its type names",
                            f.name.name
                        )),
                        false => None,
                    };
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
        };
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

    /// Function `f`, whose signature is part of a public item's interface
    /// if `interface` says what to say of a private type in it.
    fn function(&mut self, f: &mut FnDecl, interface: Option<String>) -> Result<(), Error> {
        let n = f.generics.types.len();
        self.params
            .extend(f.generics.types.iter().map(|p| p.name.clone()));
        let outer = std::mem::replace(&mut self.interface, interface);
        self.generics(&mut f.generics)?;
        for p in &mut f.params {
            self.ty(&mut p.ty)?;
        }
        self.ty(&mut f.ret)?;
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
                self.expr(pred)?;
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

    fn expr(&mut self, e: &mut Expr) -> Result<(), Error> {
        match &mut e.kind {
            ExprKind::For(f) => {
                self.pattern(&mut f.pat)?;
                self.expr(&mut f.iter)?;
                self.expr(&mut f.body)?;
                no_break_value(&f.body, f.label.as_ref().map(|l| &l.name.name), 0)?;
                let ExprKind::For(f) = std::mem::replace(&mut e.kind, ExprKind::Unit) else {
                    unreachable!("matched above");
                };
                e.kind = self.for_loop(*f, e.span);
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
            init: Some(f.iter),
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

    fn pattern(&mut self, p: &mut Pattern) -> Result<(), Error> {
        match &mut p.kind {
            PatternKind::Move(_) => unreachable!("written by lift"),
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
            PatternKind::Or(alts) => {
                for p in alts {
                    self.pattern(p)?;
                }
            }
            PatternKind::Wild
            | PatternKind::Bind(..)
            | PatternKind::Int(_)
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

/// Checks that no `break` in `e`, the body of a `for` labelled `label`, if
/// it is, gives that loop a value: a `for` can end without a `break`, so it
/// can only be `()`. `depth` is how many loops in it `e` is, which an
/// unlabelled `break` would be for instead.
fn no_break_value(e: &Expr, label: Option<&String>, depth: usize) -> Result<(), Error> {
    match &e.kind {
        ExprKind::Break(l, Some(_)) => {
            let ours = match l {
                Some(l) => Some(&l.name) == label,
                None => depth == 0,
            };
            if ours {
                return Err(Error::ty(
                    e.span,
                    "a `break` with a value only works in a `loop`: a `for` loop can end \
                     without a `break`",
                ));
            }
        }
        // A closure's body is a function of its own.
        ExprKind::Lambda(_) => return Ok(()),
        ExprKind::Loop(..) | ExprKind::While(..) | ExprKind::For(_) => {
            for c in e.children() {
                no_break_value(c, label, depth + 1)?;
            }
            return Ok(());
        }
        _ => {}
    }
    for c in e.children() {
        no_break_value(c, label, depth)?;
    }
    Ok(())
}

/// The value of `e`, if it's an integer constant: `3` or `-3`.
fn constant(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::Int(n) => Some(*n),
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
fn children_mut(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Move(_) => unreachable!("written by lift"),
        ExprKind::Int(_)
        | ExprKind::Bool(_)
        | ExprKind::Unit
        | ExprKind::Var(..)
        | ExprKind::Continue(_) => Vec::new(),
        ExprKind::Unary(_, x)
        | ExprKind::Field(x, _)
        | ExprKind::Loop(_, x)
        | ExprKind::Unsafe(x) => vec![&mut **x],
        ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter_mut().map(|x| &mut **x).collect(),
        ExprKind::Binary(_, l, r)
        | ExprKind::Assign(_, l, r)
        | ExprKind::While(_, l, r)
        | ExprKind::Range(l, r) => {
            vec![&mut **l, &mut **r]
        }
        ExprKind::For(f) => vec![&mut f.iter, &mut f.body],
        ExprKind::Call(f, args) | ExprKind::MethodCall(f, _, _, args) => {
            std::iter::once(&mut **f).chain(args.iter_mut()).collect()
        }
        ExprKind::If(c, then, els) => [&mut **c, &mut **then]
            .into_iter()
            .chain(els.as_deref_mut())
            .collect(),
        ExprKind::Block(stmts, tail) => stmts
            .iter_mut()
            .filter_map(|s| match s {
                Stmt::Let { init, .. } => init.as_mut(),
                Stmt::Expr(e) | Stmt::BlockLike(e) => Some(e),
                Stmt::Fn(_) | Stmt::Type(_) => None,
            })
            .chain(tail.as_deref_mut())
            .collect(),
        ExprKind::StructLit(_, _, inits) => inits.iter_mut().map(|i| &mut i.value).collect(),
        ExprKind::Variant(_, _, _, args, ..) => args.iter_mut().collect(),
        ExprKind::Match(scrutinee, arms) => std::iter::once(&mut **scrutinee)
            .chain(arms.iter_mut().map(|a| &mut a.body))
            .collect(),
        ExprKind::Lambda(l) => vec![&mut std::sync::Arc::make_mut(l).body],
    }
}

/// What to say of a private type in the signature of `f`, if that's part of
/// a public interface: if it's `pub`, and, for a method, `inherent`.
fn signature(f: &FnDecl, inherent: bool) -> Option<String> {
    (inherent && f.public.is_some())
        .then(|| format!("`{}` is public, but its signature names", f.name.name))
}
