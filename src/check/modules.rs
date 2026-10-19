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

use crate::ast::*;
use crate::error::{Error, Span};
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
    /// What each path with a `::` refers to, by its span: one to a type
    /// where a type is written (`a::Point`, by the span of the type), in a
    /// struct literal or pattern (by the span of its name), and one to a
    /// function where one is used (by the span of the expression).
    pub(crate) paths: HashMap<Span, Item>,
    pub(crate) imports: Vec<Import>,
    /// The symbol of each trait, by its index in `Program::traits`.
    pub(crate) trait_symbols: Vec<String>,
    /// Where each module ends: before its `}`, or at the end of the file.
    pub(crate) ends: Vec<usize>,
    /// The last segment of the path of each name a `use` brings in, and
    /// what it names: something in each namespace.
    pub(crate) leaf_items: Vec<(Ident, Vec<Item>)>,
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
        let b = match ns {
            Ns::Types => self.mods[m].types.get(name),
            Ns::Values => self.mods[m].values.get(name),
        }?;
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

    /// The binding that puts trait `symbol` in scope in module `m`, if any.
    fn trait_binding(&self, m: usize, symbol: &str) -> Option<&Binding> {
        self.mods[m]
            .types
            .values()
            .find(|b| matches!(b.item, Item::Trait(k) if self.trait_symbols[k] == symbol))
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
            ROOT => symbol.to_string(),
            _ => format!("crate::{symbol}"),
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
        mods.push(Module {
            parent: m.parent,
            prefix,
            path,
            ..Module::default()
        });
    }
    let mut r = Resolver {
        m: Modules {
            mods,
            ..Modules::default()
        },
        leaves: Vec::new(),
        states: Vec::new(),
        decls: HashMap::new(),
    };
    r.declare(prog)?;
    r.imports(prog)?;
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
    };
    w.program(prog)?;
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
                "crate" if i == 0 => ROOT,
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
                    let Some(b) = self.lookup(m, name, Ns::Types)? else {
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
}

impl Rewrite<'_> {
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
            return Ok(false);
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
    fn type_path(&mut self, name: &Ident, at: Span) -> Result<(), Error> {
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
                self.r.m.paths.insert(at, b.item);
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
            TypeExprKind::Named(name, args) => {
                for a in args.iter_mut() {
                    self.ty(a)?;
                }
                let name = Ident {
                    name: name.clone(),
                    span: t.span,
                };
                self.type_path(&name, t.span)?;
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => {}
            TypeExprKind::Fn(params, ret) => {
                for p in params {
                    self.ty(p)?;
                }
                self.ty(ret)?;
            }
            TypeExprKind::Ref(_, _, inner) => self.ty(inner)?,
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
                    self.r.m.paths.insert(t.span, b.item);
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
            ExprKind::Var(name, targs) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                if name.contains("::") {
                    let name = Ident {
                        name: name.clone(),
                        span: e.span,
                    };
                    let b = self.path(&name, Ns::Values)?;
                    self.r.m.paths.insert(e.span, b.item);
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
                    };
                    let b = self.path(&path, Ns::Values)?;
                    self.r.m.paths.insert(path.span, b.item);
                    let var = Expr {
                        kind: ExprKind::Var(path.name, Vec::new()),
                        span: path.span,
                    };
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
                            self.r.m.paths.insert(ty.span, b.item);
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
                let at = ty.span;
                self.type_path(&ty.clone(), at)?;
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

    fn pattern(&mut self, p: &mut Pattern) -> Result<(), Error> {
        match &mut p.kind {
            PatternKind::Variant(ty, targs, _, pats) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                let at = ty.span;
                self.type_path(&ty.clone(), at)?;
                for p in pats {
                    self.pattern(p)?;
                }
            }
            PatternKind::Struct(ty, targs, fields, _) => {
                for t in targs.iter_mut() {
                    self.ty(t)?;
                }
                let at = ty.span;
                self.type_path(&ty.clone(), at)?;
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
        }),
        TypeExprKind::Proj(inner, None, name) => {
            let inner = plain_path(inner)?;
            Some(Ident {
                name: format!("{}::{}", inner.name, name.name),
                span: inner.span.to(name.span),
            })
        }
        _ => None,
    }
}

/// The expressions directly in `e`, to change, as `Expr::children` gives
/// them.
fn children_mut(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Int(_)
        | ExprKind::Bool(_)
        | ExprKind::Unit
        | ExprKind::Var(..)
        | ExprKind::Continue(_) => Vec::new(),
        ExprKind::Unary(_, x) | ExprKind::Field(x, _) | ExprKind::Loop(_, x) => vec![&mut **x],
        ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter_mut().map(|x| &mut **x).collect(),
        ExprKind::Binary(_, l, r) | ExprKind::Assign(_, l, r) | ExprKind::While(_, l, r) => {
            vec![&mut **l, &mut **r]
        }
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
