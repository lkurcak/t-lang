//! Which struct or enum each name in a type refers to. A block can declare
//! types of its own, *local types*, which like nested functions mean what
//! they would at the top level, are in scope in the whole block, and shadow
//! those of the same name outside it. Nothing else can shadow a type, and
//! what a type name refers to doesn't depend on inference, so it's all
//! worked out here, before checking starts, for the whole program.
//!
//! Like a function, each type has a symbol, unique in the program: its name
//! at the crate root, after its module's path in a module (`shapes::Point`),
//! or if it's local, the symbol of the function it's in and its own name,
//! like `main::Point` (see `check/scope.rs`). Lifting moves local types to
//! the top level under them (see `lift.rs`).
//!
//! At the level of a module, a name means what the module's namespace says
//! (see `check/modules.rs`), and a path with a `::` what resolving it said.

use super::modules::{Item, Modules};
use super::scope::{FnId, symbol};
use crate::ast::*;
use std::collections::{HashMap, HashSet};

/// A struct or enum's definition, besides what it's made of.
pub(super) struct TypeDef {
    pub(super) symbol: String,
    /// The function it's declared in, if it's local.
    pub(super) parent: Option<FnId>,
    /// The block it's declared in, as a number unique in the program: 0
    /// for the top level.
    pub(super) block: usize,
    /// The module it's in, or that the function it's local to is in.
    pub(super) module: usize,
    /// Its fields that aren't `pub`: none for an enum, whose variants are
    /// public wherever it is.
    pub(super) private: Vec<String>,
}

/// What a name in a type refers to, where it's written: a struct or enum,
/// by its symbol, or a type parameter it can't use. One that isn't here is
/// a type parameter of what it's written in, or `int` or `bool`, or nothing.
#[derive(Debug, Clone)]
pub(super) enum Scoped {
    Adt(String),
    /// A type parameter of a function that what it's written in is nested
    /// in. With the keyword and name of what it's written in.
    OuterParam(&'static str, String),
}

/// Every type's definition, in the order of `Program::all_types`, and what
/// every name in a type refers to, by its node (a `TypeExpr`, or the
/// `Ident` of a struct literal, pattern or declaration): in types written anywhere,
/// struct literals and patterns, and where types are declared.
pub(super) fn names(
    prog: &Program,
    fns: &[(&FnDecl, Option<usize>)],
    fn_symbols: &[String],
    modules: &Modules,
) -> (Vec<TypeDef>, HashMap<NodeId, Scoped>) {
    // The symbols of the types in modules come first, so no local type's
    // can be one of them.
    let top: Vec<String> = prog
        .types
        .iter()
        .map(|d| format!("{}{}", modules.mods[d.module()].prefix, d.name().name))
        .collect();
    let mut w = Walk {
        modules,
        top: top.clone(),
        levels: Vec::new(),
        items: Vec::new(),
        fn_ids: fns
            .iter()
            .enumerate()
            .map(|(i, &(f, _))| (f.name.id, FnId(i)))
            .collect(),
        fn_symbols,
        current: Vec::new(),
        taken: top.iter().cloned().collect(),
        blocks: 0,
        defs: HashMap::new(),
        names: HashMap::new(),
    };
    w.declare(prog.types.iter().map(|d| (d, None)).collect());
    for f in &prog.fns {
        w.module(f.module);
        w.function(f, &[]);
    }
    // A method can use its impl's type parameters as well as its own.
    for d in &prog.impls {
        w.module(d.module);
        let params: Vec<String> = d.generics.types.iter().map(|p| p.name.clone()).collect();
        w.levels.push(Level::Params(params.clone(), w.items.len()));
        w.items.push(("impl", "impl".into()));
        w.ty(&d.self_ty);
        w.bounds(&d.generics);
        for a in &d.assoc {
            w.ty(&a.ty);
        }
        w.leave();
        for f in &d.methods {
            w.function(f, &params);
        }
    }
    for t in &prog.traits {
        w.module(t.module);
        for m in &t.methods {
            match m.provided {
                true => w.function(&m.decl, &[]),
                // No body, so not a function of its own: just its types.
                false => {
                    w.enter("fn", &m.decl.name, &m.decl.generics.types);
                    w.bounds(&m.decl.generics);
                    m.decl.params.iter().for_each(|p| w.ty(&p.ty));
                    w.ty(&m.decl.ret);
                    w.leave();
                }
            }
        }
    }
    let defs = prog
        .all_types()
        .into_iter()
        .map(|(d, _)| w.defs.remove(&d.name().id).expect("every type is walked"))
        .collect();
    (defs, w.names)
}

/// A scope, in the stack of them that a type name is looked up in.
enum Level {
    /// That of a module: what its namespace says. Always at the bottom.
    Module(usize),
    /// The types the top level or a block declares, as name and symbol.
    Types(Vec<(String, String)>),
    /// The type parameters of a function, struct or enum, and where it is
    /// in `Walk::items`.
    Params(Vec<String>, usize),
}

struct Walk<'a> {
    modules: &'a Modules,
    /// The symbol of each type in `Program::types`.
    top: Vec<String>,
    /// The scopes around what's being walked, innermost last.
    levels: Vec<Level>,
    /// The functions and types being walked, innermost last, as keyword and
    /// name.
    items: Vec<(&'static str, String)>,
    /// Each function's id, and below each type's definition, by the node
    /// of its name.
    fn_ids: HashMap<NodeId, FnId>,
    fn_symbols: &'a [String],
    /// The functions being walked, innermost last.
    current: Vec<FnId>,
    /// The symbols given to local types so far.
    taken: HashSet<String>,
    /// How many blocks have declared types so far.
    blocks: usize,
    defs: HashMap<NodeId, TypeDef>,
    names: HashMap<NodeId, Scoped>,
}

impl Walk<'_> {
    /// What type name `name` refers to here.
    fn lookup(&self, name: &str) -> Option<Scoped> {
        for level in self.levels.iter().rev() {
            match level {
                Level::Module(m) => {
                    let k = self.modules.type_named(*m, name)?;
                    return Some(Scoped::Adt(self.top[k].clone()));
                }
                Level::Types(types) => {
                    if let Some((_, symbol)) = types.iter().find(|(n, _)| n == name) {
                        return Some(Scoped::Adt(symbol.clone()));
                    }
                }
                Level::Params(params, item) if params.iter().any(|p| p == name) => {
                    if *item + 1 == self.items.len() {
                        return None;
                    }
                    let (keyword, inner) = self.items.last().expect("inside the item");
                    return Some(Scoped::OuterParam(keyword, inner.clone()));
                }
                Level::Params(..) => {}
            }
        }
        None
    }

    /// Starts walking the items of module `m`.
    fn module(&mut self, m: usize) {
        self.levels.clear();
        self.levels.push(Level::Module(m));
    }

    /// Notes what `name`, written as node `id`, refers to. A path with a
    /// `::` refers to what resolving it said, if it's a struct or enum.
    fn name(&mut self, name: &str, id: NodeId) {
        if name.contains("::") {
            if let Some(Item::Type(k)) = self.modules.paths.get(&id) {
                self.names.insert(id, Scoped::Adt(self.top[*k].clone()));
            }
            return;
        }
        if let Some(s) = self.lookup(name) {
            self.names.insert(id, s);
        }
    }

    /// Opens the scope of the types `decls`, declared together, in the
    /// function each is local to, if any, and walks them.
    fn declare(&mut self, decls: Vec<(&TypeDecl, Option<FnId>)>) {
        let block = match decls.first() {
            Some((_, Some(_))) => {
                self.blocks += 1;
                self.blocks
            }
            _ => 0,
        };
        let mut level = Vec::new();
        for (k, &(d, parent)) in decls.iter().enumerate() {
            let name = d.name();
            let symbol = match parent {
                None => self.top[k].clone(),
                Some(f) => symbol(&self.fn_symbols[f.0], &name.name, &self.taken),
            };
            self.taken.insert(symbol.clone());
            self.names.insert(name.id, Scoped::Adt(symbol.clone()));
            level.push((name.name.clone(), symbol.clone()));
            let private = match d {
                TypeDecl::Struct(s) => s
                    .fields
                    .iter()
                    .filter(|f| f.public.is_none())
                    .map(|f| f.name.name.clone())
                    .collect(),
                TypeDecl::Enum(_) => Vec::new(),
            };
            let def = TypeDef {
                symbol,
                parent,
                block,
                module: d.module(),
                private,
            };
            self.defs.insert(name.id, def);
        }
        // The top-level ones are in their modules' namespaces instead.
        if decls.first().is_some_and(|(_, parent)| parent.is_none()) {
            for (d, _) in decls {
                self.module(d.module());
                self.type_decl(d);
            }
            return;
        }
        self.levels.push(Level::Types(level));
        for (d, _) in decls {
            self.type_decl(d);
        }
    }

    /// Walks `f`, with what's in it. Each item opens a scope for its type
    /// parameters, which the items in it can't use. A method's has its
    /// impl's, `outer`, too.
    fn function(&mut self, f: &FnDecl, outer: &[String]) {
        self.current.push(self.fn_ids[&f.name.id]);
        self.enter("fn", &f.name, &f.generics.types);
        if let Some(Level::Params(params, _)) = self.levels.last_mut() {
            params.extend(outer.iter().cloned());
        }
        self.bounds(&f.generics);
        for p in &f.params {
            self.ty(&p.ty);
        }
        self.ty(&f.ret);
        self.expr(&f.body);
        self.leave();
        self.current.pop();
    }

    /// The types in the bounds of `g`, like `int` in `T: Iter<Item = int>`
    /// and `F: Fn(int) -> bool`.
    fn bounds(&mut self, g: &Generics) {
        for (_, items) in &g.bounds {
            for t in items.iter().flat_map(|b| b.types()) {
                self.ty(t);
            }
        }
    }

    fn type_decl(&mut self, d: &TypeDecl) {
        match d {
            TypeDecl::Struct(s) => {
                self.enter("struct", &s.name, &s.generics.types);
                for f in &s.fields {
                    self.ty(&f.ty);
                }
            }
            TypeDecl::Enum(e) => {
                self.enter("enum", &e.name, &e.generics.types);
                for t in e.variants.iter().flat_map(|v| &v.fields) {
                    self.ty(t);
                }
            }
        }
        self.bounds(d.generics());
        self.leave();
    }

    fn enter(&mut self, keyword: &'static str, name: &Ident, params: &[Ident]) {
        self.items.push((keyword, name.name.clone()));
        let params = params.iter().map(|p| p.name.clone()).collect();
        self.levels
            .push(Level::Params(params, self.items.len() - 1));
    }

    fn leave(&mut self) {
        self.levels.pop();
        self.items.pop();
    }

    fn ty(&mut self, t: &TypeExpr) {
        match &t.kind {
            TypeExprKind::Named(name, args) => {
                self.name(name, t.id);
                args.iter().for_each(|a| self.ty(a));
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => {}
            TypeExprKind::Fn(params, ret, effects) => {
                params.iter().for_each(|p| self.ty(p));
                self.ty(ret);
                effects
                    .iter()
                    .flat_map(EffectSet::types)
                    .for_each(|t| self.ty(t));
            }
            TypeExprKind::Ref(_, _, inner) | TypeExprKind::Proj(inner, ..) => self.ty(inner),
            TypeExprKind::Impl(bounds) => {
                for t in bounds.iter().flat_map(BoundItem::types) {
                    self.ty(t);
                }
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Var(_, targs) => targs.iter().for_each(|t| self.ty(t)),
            ExprKind::StructLit(name, targs, _) | ExprKind::Variant(name, targs, ..) => {
                self.name(&name.name, name.id);
                targs.iter().for_each(|t| self.ty(t));
            }
            ExprKind::Block(stmts, tail) => {
                let f = *self.current.last().expect("blocks are in functions");
                let decls = stmts.iter().filter_map(Stmt::as_type);
                self.declare(decls.map(|d| (d, Some(f))).collect());
                for stmt in stmts {
                    match stmt {
                        Stmt::Let { pat, ty, init, .. } => {
                            self.pattern(pat);
                            ty.iter().for_each(|t| self.ty(t));
                            init.iter().for_each(|e| self.expr(e));
                        }
                        Stmt::Expr(e) | Stmt::BlockLike(e) => self.expr(e),
                        Stmt::Fn(f) => self.function(f, &[]),
                        // Walked by `declare`.
                        Stmt::Type(_) => {}
                    }
                }
                tail.iter().for_each(|t| self.expr(t));
                self.levels.pop();
                return;
            }
            ExprKind::Match(scrutinee, arms) => {
                self.expr(scrutinee);
                for arm in arms {
                    self.pattern(&arm.pat);
                    self.expr(&arm.body);
                }
                return;
            }
            ExprKind::Lambda(l) => {
                l.params.iter().flat_map(|p| &p.ty).for_each(|t| self.ty(t));
                l.ret.iter().for_each(|t| self.ty(t));
            }
            _ => {}
        }
        for c in e.children() {
            self.expr(c);
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Move(_) => unreachable!("written by lift"),
            PatternKind::Variant(name, targs, _, pats) => {
                self.name(&name.name, name.id);
                targs.iter().for_each(|t| self.ty(t));
                pats.iter().for_each(|p| self.pattern(p));
            }
            PatternKind::Struct(name, targs, fields, _) => {
                self.name(&name.name, name.id);
                targs.iter().for_each(|t| self.ty(t));
                fields.iter().for_each(|f| self.pattern(&f.pat));
            }
            PatternKind::Or(alts) => alts.iter().for_each(|p| self.pattern(p)),
            PatternKind::Wild
            | PatternKind::Bind(..)
            | PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
    }
}
