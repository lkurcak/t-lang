//! Which struct or enum each name in a type refers to. A block can declare
//! types of its own, *local types*, which like nested functions mean what
//! they would at the top level, are in scope in the whole block, and shadow
//! those of the same name outside it. Nothing else can shadow a type, and
//! what a type name refers to doesn't depend on inference, so it's all
//! worked out here, before checking starts, for the whole program.
//!
//! Like a function, each type has a symbol, unique in the program: its name
//! at the top level, or if it's local, the symbol of the function it's in
//! and its own name, like `main::Point` (see `check/scope.rs`). Lifting
//! moves local types to the top level under them (see `lift.rs`).

use super::scope::{FnId, symbol};
use crate::ast::*;
use crate::error::Span;
use std::collections::{HashMap, HashSet};

/// A struct or enum's definition, besides what it's made of.
pub(super) struct TypeDef {
    pub(super) symbol: String,
    /// The function it's declared in, if it's local.
    pub(super) parent: Option<FnId>,
    /// The block it's declared in, as a number unique in the program: 0
    /// for the top level.
    pub(super) block: usize,
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
/// every name in a type refers to, by its span: in types written anywhere,
/// struct literals and patterns, and where types are declared.
pub(super) fn names(
    prog: &Program,
    fns: &[(&FnDecl, Option<usize>)],
    fn_symbols: &[String],
) -> (Vec<TypeDef>, HashMap<Span, Scoped>) {
    let mut w = Walk {
        levels: Vec::new(),
        items: Vec::new(),
        fn_ids: fns
            .iter()
            .enumerate()
            .map(|(i, &(f, _))| (std::ptr::from_ref(f), FnId(i)))
            .collect(),
        fn_symbols,
        current: Vec::new(),
        taken: HashSet::new(),
        blocks: 0,
        defs: HashMap::new(),
        names: HashMap::new(),
    };
    let top = prog.types.iter().map(|d| (d, None)).collect();
    w.declare(top);
    for f in &prog.fns {
        w.function(f);
    }
    w.levels.pop();
    let defs = prog
        .all_types()
        .into_iter()
        .map(|(d, _)| {
            w.defs
                .remove(&std::ptr::from_ref(d))
                .expect("every type is walked")
        })
        .collect();
    (defs, w.names)
}

/// A scope, in the stack of them that a type name is looked up in.
enum Level {
    /// The types the top level or a block declares, as name and symbol.
    Types(Vec<(String, String)>),
    /// The type parameters of a function, struct or enum, and where it is
    /// in `Walk::items`.
    Params(Vec<String>, usize),
}

struct Walk<'a> {
    /// The scopes around what's being walked, innermost last.
    levels: Vec<Level>,
    /// The functions and types being walked, innermost last, as keyword and
    /// name.
    items: Vec<(&'static str, String)>,
    /// Each function's id, and below each type's definition, by which
    /// declaration it is. Not by where: monomorphisation makes copies with
    /// the same spans, and those are checked too.
    fn_ids: HashMap<*const FnDecl, FnId>,
    fn_symbols: &'a [String],
    /// The functions being walked, innermost last.
    current: Vec<FnId>,
    /// The symbols given to local types so far.
    taken: HashSet<String>,
    /// How many blocks have declared types so far.
    blocks: usize,
    defs: HashMap<*const TypeDecl, TypeDef>,
    names: HashMap<Span, Scoped>,
}

impl Walk<'_> {
    /// What type name `name` refers to here.
    fn lookup(&self, name: &str) -> Option<Scoped> {
        for level in self.levels.iter().rev() {
            match level {
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

    /// Notes what `name`, written at `span`, refers to.
    fn name(&mut self, name: &str, span: Span) {
        if let Some(s) = self.lookup(name) {
            self.names.insert(span, s);
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
        for &(d, parent) in &decls {
            let name = d.name();
            let symbol = match parent {
                None => name.name.clone(),
                Some(f) => symbol(&self.fn_symbols[f.0], &name.name, &self.taken),
            };
            self.taken.insert(symbol.clone());
            self.names.insert(name.span, Scoped::Adt(symbol.clone()));
            level.push((name.name.clone(), symbol.clone()));
            let def = TypeDef {
                symbol,
                parent,
                block,
            };
            self.defs.insert(std::ptr::from_ref(d), def);
        }
        self.levels.push(Level::Types(level));
        for (d, _) in decls {
            self.type_decl(d);
        }
    }

    /// Walks `f`, with what's in it. Each item opens a scope for its type
    /// parameters, which the items in it can't use.
    fn function(&mut self, f: &FnDecl) {
        self.current.push(self.fn_ids[&std::ptr::from_ref(f)]);
        self.enter("fn", &f.name, &f.type_params);
        for p in &f.params {
            self.ty(&p.ty);
        }
        self.ty(&f.ret);
        self.expr(&f.body);
        self.leave();
        self.current.pop();
    }

    fn type_decl(&mut self, d: &TypeDecl) {
        match d {
            TypeDecl::Struct(s) => {
                self.enter("struct", &s.name, &s.type_params);
                for f in &s.fields {
                    self.ty(&f.ty);
                }
            }
            TypeDecl::Enum(e) => {
                self.enter("enum", &e.name, &e.type_params);
                for t in e.variants.iter().flat_map(|v| &v.fields) {
                    self.ty(t);
                }
            }
        }
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
                self.name(name, t.span);
                args.iter().for_each(|a| self.ty(a));
            }
            TypeExprKind::Unit => {}
            TypeExprKind::Fn(params, ret) => {
                params.iter().for_each(|p| self.ty(p));
                self.ty(ret);
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Var(_, targs) => targs.iter().for_each(|t| self.ty(t)),
            ExprKind::StructLit(name, targs, _) | ExprKind::Variant(name, targs, ..) => {
                self.name(&name.name, name.span);
                targs.iter().for_each(|t| self.ty(t));
            }
            ExprKind::Block(stmts, tail) => {
                let f = *self.current.last().expect("blocks are in functions");
                let decls = stmts.iter().filter_map(Stmt::as_type);
                self.declare(decls.map(|d| (d, Some(f))).collect());
                for stmt in stmts {
                    match stmt {
                        Stmt::Let { pat, ty, init } => {
                            self.pattern(pat);
                            ty.iter().for_each(|t| self.ty(t));
                            self.expr(init);
                        }
                        Stmt::Expr(e) | Stmt::BlockLike(e) => self.expr(e),
                        Stmt::Fn(f) => self.function(f),
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
            PatternKind::Variant(name, targs, _, pats) => {
                self.name(&name.name, name.span);
                targs.iter().for_each(|t| self.ty(t));
                pats.iter().for_each(|p| self.pattern(p));
            }
            PatternKind::Struct(name, targs, fields, _) => {
                self.name(&name.name, name.span);
                targs.iter().for_each(|t| self.ty(t));
                fields.iter().for_each(|f| self.pattern(&f.pat));
            }
            PatternKind::Or(alts) => alts.iter().for_each(|p| self.pattern(p)),
            PatternKind::Wild
            | PatternKind::Bind(_)
            | PatternKind::Int(_)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Unit => {}
        }
    }
}
