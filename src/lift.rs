//! Lifting: moves every nested function and local type to the top level,
//! named by its symbol (`main::helper`, `main::Point`, see `check/scope.rs`
//! and `check/names.rs`), and renames every use of a function or type in
//! the declarations to the symbol the checker found it refers to. A nested
//! function can't capture anything, and a local type can't use the type
//! parameters of the function it's in, so that's all it takes: each means
//! the same at the top level, under a name of its own. After this, nothing
//! needs to know there were nested functions or local types, or that two of
//! them can share a name: not monomorphisation, not the interpreter.
//!
//! What's lifted is declarations: what each function does is its MIR,
//! which the borrow checker built from its body, checked, and kept
//! (`Info::mir`), so a lifted function's body is empty. Monomorphisation
//! copies the MIR.

use crate::ast::*;
use crate::check::Info;
use std::cell::Cell;

/// A checked program with no nested functions or local types, paired with
/// its semantic information for monomorphisation.
pub struct Lifted<'a> {
    pub(crate) program: Program,
    pub(crate) info: &'a Info,
}

/// `checked` with no nested functions or local types.
/// Each function nested in another comes right after it, in
/// order, and the local types come after the top-level ones, in the order of
/// `Program::all_types`.
pub fn lift(checked: &crate::CheckedProgram) -> Lifted<'_> {
    let (prog, info) = (checked.program(), checked.info());
    Lifted {
        program: lift_program(prog, info),
        info,
    }
}

fn lift_program(prog: &Program, info: &Info) -> Program {
    let cx = Cx {
        info,
        next_id: Cell::new(prog.next_id),
    };
    let types: Vec<TypeDecl> = prog
        .all_types()
        .into_iter()
        .map(|(d, _)| cx.type_decl(d))
        .collect();
    let mut fns = Vec::new();
    for f in &prog.fns {
        cx.function(f, &mut fns);
    }
    let impls: Vec<ImplDecl> = prog
        .impls
        .iter()
        .map(|d| ImplDecl {
            self_ty: cx.ty(&d.self_ty),
            generics: cx.generics(&d.generics),
            assoc: d
                .assoc
                .iter()
                .map(|a| AssocDef {
                    ty: cx.ty(&a.ty),
                    ..a.clone()
                })
                .collect(),
            methods: d.methods.iter().map(|f| cx.method(f, &mut fns)).collect(),
            ..d.clone()
        })
        .collect();
    let traits: Vec<TraitDecl> = prog
        .traits
        .iter()
        .map(|t| TraitDecl {
            methods: t
                .methods
                .iter()
                .map(|m| TraitMethod {
                    decl: match m.provided {
                        true => cx.method(&m.decl, &mut fns),
                        false => m.decl.clone(),
                    },
                    provided: m.provided,
                })
                .collect(),
            ..t.clone()
        })
        .collect();
    // Everything is at the top level now, named by its symbol.
    let types = types.into_iter().map(TypeDecl::at_root).collect::<Vec<_>>();
    Program {
        mods: prog.mods[..1].to_vec(),
        core: None,
        libraries: Vec::new(),
        packages: Vec::new(),
        uses: Vec::new(),
        types,
        traits: traits
            .into_iter()
            .map(|t| TraitDecl { module: ROOT, ..t })
            .collect(),
        impls: impls
            .into_iter()
            .map(|d| ImplDecl { module: ROOT, ..d })
            .collect(),
        fns,
        stray_docs: prog.stray_docs.clone(),
        next_id: cx.next_id.get(),
        proved: info.proved.clone(),
        measures: info.measures.clone(),
    }
}

struct Cx<'p> {
    info: &'p Info,
    /// `Program::next_id`: the nodes written here that weren't in the
    /// program take ids from it. The others keep theirs.
    next_id: Cell<u32>,
}

impl<'p> Cx<'p> {
    /// Adds `f`, named by its symbol, to `out`, then those nested in it.
    fn function(&self, f: &'p FnDecl, out: &mut Vec<FnDecl>) {
        let (decl, nested) = self.lifted(f);
        out.push(decl);
        for g in nested {
            self.function(g, out);
        }
    }

    /// Method `f`, named by its symbol, adding the functions nested in it
    /// to `out`.
    fn method(&self, f: &'p FnDecl, out: &mut Vec<FnDecl>) -> FnDecl {
        let (decl, nested) = self.lifted(f);
        for g in nested {
            self.function(g, out);
        }
        decl
    }

    /// `f`, named by its symbol, and the functions nested in it.
    fn lifted(&self, f: &'p FnDecl) -> (FnDecl, Vec<&'p FnDecl>) {
        let mut nested = Vec::new();
        nested_fns(&f.body, &mut nested);
        // What it does is its MIR, which the borrow checker kept.
        let body = Expr {
            kind: ExprKind::Block(Vec::new(), None),
            span: f.body.span,
            id: f.body.id,
        };
        let decl = FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            module: ROOT,
            vis: f.vis,
            name: self.ident(&f.name),
            generics: self.generics(&f.generics),
            params: f.params.iter().map(|p| self.param(p)).collect(),
            parens: f.parens,
            ret: self.ty(&f.ret),
            effects: f.effects.as_ref().map(|e| self.effects(e)),
            body,
            intrinsic: f.intrinsic,
            measure: f.measure,
            test: f.test,
            ensures: f.ensures.clone(),
        };
        (decl, nested)
    }

    fn type_decl(&self, d: &TypeDecl) -> TypeDecl {
        match d {
            TypeDecl::Struct(s) => TypeDecl::Struct(StructDecl {
                doc: s.doc.clone(),
                allows: s.allows.clone(),
                span: s.span,
                module: s.module,
                vis: s.vis,
                name: self.ident(&s.name),
                generics: self.generics(&s.generics),
                derives: s.derives.clone(),
                fields: s.fields.iter().map(|f| self.param(f)).collect(),
                intrinsic: s.intrinsic,
            }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl {
                doc: e.doc.clone(),
                allows: e.allows.clone(),
                span: e.span,
                module: e.module,
                vis: e.vis,
                name: self.ident(&e.name),
                generics: self.generics(&e.generics),
                derives: e.derives.clone(),
                variants: e
                    .variants
                    .iter()
                    .map(|v| Variant {
                        doc: v.doc.clone(),
                        allows: v.allows.clone(),
                        span: v.span,
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|t| self.ty(t)).collect(),
                    })
                    .collect(),
            }),
            TypeDecl::Alias(a) => TypeDecl::Alias(AliasDecl {
                doc: a.doc.clone(),
                allows: a.allows.clone(),
                span: a.span,
                module: a.module,
                vis: a.vis,
                name: self.ident(&a.name),
                generics: self.generics(&a.generics),
                ty: self.ty(&a.ty),
            }),
        }
    }

    /// `id`, renamed to the symbol of what it refers to, if it's a function
    /// or type.
    fn ident(&self, id: &Ident) -> Ident {
        Ident {
            name: self.renamed(&id.name, id.id),
            span: id.span,
            id: id.id,
        }
    }

    /// The symbol of what `name`, written as node `at`, refers to, or
    /// `name` if it isn't a function or type.
    fn renamed(&self, name: &str, at: NodeId) -> String {
        let symbol = self.info.symbols.get(&at);
        symbol.map_or_else(|| name.to_string(), String::clone)
    }

    fn param(&self, p: &Param) -> Param {
        Param {
            doc: p.doc.clone(),
            allows: p.allows.clone(),
            mutable: p.mutable,
            vis: p.vis,
            name: p.name.clone(),
            ty: self.ty(&p.ty),
        }
    }

    fn ty(&self, t: &TypeExpr) -> TypeExpr {
        let kind = match &t.kind {
            TypeExprKind::Named(name, args) => {
                TypeExprKind::Named(self.renamed(name, t.id), self.types(args))
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => t.kind.clone(),
            TypeExprKind::Fn(params, ret, effects) => TypeExprKind::Fn(
                self.types(params),
                Box::new(self.ty(ret)),
                effects.as_ref().map(|e| self.effects(e)),
            ),
            TypeExprKind::Ref(lifetime, mutable, inner) => {
                TypeExprKind::Ref(lifetime.clone(), *mutable, Box::new(self.ty(inner)))
            }
            TypeExprKind::Slice(inner) => TypeExprKind::Slice(Box::new(self.ty(inner))),
            TypeExprKind::Array(inner, n) => {
                TypeExprKind::Array(Box::new(self.ty(inner)), Box::new(self.ty(n)))
            }
            TypeExprKind::Nat(_) => t.kind.clone(),
            TypeExprKind::Proj(inner, tr, name) => {
                TypeExprKind::Proj(Box::new(self.ty(inner)), tr.clone(), name.clone())
            }
            TypeExprKind::Refined(v, base, pred) => {
                TypeExprKind::Refined(v.clone(), Box::new(self.ty(base)), pred.clone())
            }
            TypeExprKind::Impl(bounds) => {
                let mut bounds = bounds.clone();
                for t in bounds.iter_mut().flat_map(|b| b.types_mut()) {
                    *t = self.ty(t);
                }
                TypeExprKind::Impl(bounds)
            }
        };
        TypeExpr {
            kind,
            span: t.span,
            id: t.id,
        }
    }

    /// `g`, with the types in its bounds renamed.
    fn generics(&self, g: &Generics) -> Generics {
        let mut g = g.clone();
        // Checked already, and only about types that monomorphising
        // replaces.
        g.wheres.clear();
        g.where_span = None;
        for (_, items) in &mut g.bounds {
            for t in items.iter_mut().flat_map(|b| b.types_mut()) {
                *t = self.ty(t);
            }
        }
        g
    }

    /// `e`, with the types whose associated effects it names renamed.
    fn effects(&self, e: &EffectSet) -> EffectSet {
        let mut e = e.clone();
        for t in e.types_mut() {
            *t = self.ty(t);
        }
        e
    }

    fn types(&self, ts: &[TypeExpr]) -> Vec<TypeExpr> {
        ts.iter().map(|t| self.ty(t)).collect()
    }
}

/// Adds the functions nested in `e` to `out`, in order. Not those nested in
/// those: lifting each finds its own.
fn nested_fns<'p>(e: &'p Expr, out: &mut Vec<&'p FnDecl>) {
    if let ExprKind::Block(stmts, _) = &e.kind {
        for s in stmts {
            if let Stmt::Fn(f) = s {
                out.push(f);
            }
        }
    }
    for c in e.children() {
        nested_fns(c, out);
    }
}
