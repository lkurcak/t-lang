//! Lifting: moves every nested function and local type to the top level,
//! named by its symbol (`main::helper`, `main::Point`, see `check/scope.rs`
//! and `check/names.rs`), and renames every use of a function or type to
//! the symbol the checker found it refers to. A nested function can't
//! capture anything, and a local type can't use the type parameters of the
//! function it's in, so that's all it takes: each means the same at the top
//! level, under a name of its own. After this, nothing needs to know there
//! were nested functions or local types, or that two of them can share a
//! name: not monomorphisation, not the interpreter, and not a backend.
//!
//! It also writes out the references the checker made into others
//! (*coercions*, see `Checker::coerce`): `r` used as a `&T` when it's a
//! `&Box<T>` becomes `&**r`, so what comes after needn't know about them.

use crate::ast::*;
use crate::check::Info;
use crate::error::Span;
use std::sync::Arc;

/// `prog`, which must have checked with `info`, with no nested functions or
/// local types. Each function nested in another comes right after it, in
/// order, and the local types come after the top-level ones, in the order of
/// `Program::all_types`.
pub fn lift(prog: &Program, info: &Info) -> Program {
    let cx = Cx { info };
    let types = prog
        .all_types()
        .into_iter()
        .map(|(d, _)| cx.type_decl(d))
        .collect();
    let mut fns = Vec::new();
    for f in &prog.fns {
        cx.function(f, &mut fns);
    }
    Program {
        types,
        fns,
        stray_docs: prog.stray_docs.clone(),
    }
}

struct Cx<'p> {
    info: &'p Info,
}

impl<'p> Cx<'p> {
    /// Adds `f`, named by its symbol, to `out`, then those nested in it.
    fn function(&self, f: &'p FnDecl, out: &mut Vec<FnDecl>) {
        let mut nested = Vec::new();
        let body = self.expr(&f.body, &mut nested);
        out.push(FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            name: self.ident(&f.name),
            generics: f.generics.clone(),
            params: f.params.iter().map(|p| self.param(p)).collect(),
            parens: f.parens,
            ret: self.ty(&f.ret),
            body,
        });
        for g in nested {
            self.function(g, out);
        }
    }

    fn type_decl(&self, d: &TypeDecl) -> TypeDecl {
        match d {
            TypeDecl::Struct(s) => TypeDecl::Struct(StructDecl {
                doc: s.doc.clone(),
                allows: s.allows.clone(),
                span: s.span,
                name: self.ident(&s.name),
                generics: s.generics.clone(),
                derives: s.derives.clone(),
                fields: s.fields.iter().map(|f| self.param(f)).collect(),
            }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl {
                doc: e.doc.clone(),
                allows: e.allows.clone(),
                span: e.span,
                name: self.ident(&e.name),
                generics: e.generics.clone(),
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
        }
    }

    /// `id`, renamed to the symbol of what it refers to, if it's a function
    /// or type.
    fn ident(&self, id: &Ident) -> Ident {
        Ident {
            name: self.renamed(&id.name, id.span),
            span: id.span,
        }
    }

    /// The symbol of what `name`, written at `at`, refers to, or `name` if
    /// it isn't a function or type.
    fn renamed(&self, name: &str, at: Span) -> String {
        let symbol = self.info.symbols.get(&at);
        symbol.map_or_else(|| name.to_string(), String::clone)
    }

    fn param(&self, p: &Param) -> Param {
        Param {
            doc: p.doc.clone(),
            allows: p.allows.clone(),
            mutable: p.mutable,
            name: p.name.clone(),
            ty: self.ty(&p.ty),
        }
    }

    fn ty(&self, t: &TypeExpr) -> TypeExpr {
        let kind = match &t.kind {
            TypeExprKind::Named(name, args) => {
                TypeExprKind::Named(self.renamed(name, t.span), self.types(args))
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => t.kind.clone(),
            TypeExprKind::Fn(params, ret) => {
                TypeExprKind::Fn(self.types(params), Box::new(self.ty(ret)))
            }
            TypeExprKind::Ref(lifetime, mutable, inner) => {
                TypeExprKind::Ref(lifetime.clone(), *mutable, Box::new(self.ty(inner)))
            }
        };
        TypeExpr { kind, span: t.span }
    }

    fn types(&self, ts: &[TypeExpr]) -> Vec<TypeExpr> {
        ts.iter().map(|t| self.ty(t)).collect()
    }

    /// `e`, renamed, adding the functions nested in it to `nested`, in
    /// order. Local types are taken out too: `lift` has them already.
    fn expr(&self, e: &'p Expr, nested: &mut Vec<&'p FnDecl>) -> Expr {
        let kind = match &e.kind {
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit | ExprKind::Continue(_) => {
                e.kind.clone()
            }
            ExprKind::Var(name, targs) => {
                ExprKind::Var(self.renamed(name, e.span), self.types(targs))
            }
            ExprKind::Unary(op, x) => ExprKind::Unary(*op, Box::new(self.expr(x, nested))),
            ExprKind::Binary(op, l, r) => ExprKind::Binary(
                *op,
                Box::new(self.expr(l, nested)),
                Box::new(self.expr(r, nested)),
            ),
            ExprKind::Call(f, args) => ExprKind::Call(
                Box::new(self.expr(f, nested)),
                args.iter().map(|a| self.expr(a, nested)).collect(),
            ),
            ExprKind::If(c, then, els) => ExprKind::If(
                Box::new(self.expr(c, nested)),
                Box::new(self.expr(then, nested)),
                els.as_ref().map(|e| Box::new(self.expr(e, nested))),
            ),
            ExprKind::Block(stmts, tail) => ExprKind::Block(
                stmts
                    .iter()
                    .filter_map(|s| {
                        Some(match s {
                            Stmt::Let {
                                pat,
                                ty,
                                init,
                                span,
                            } => Stmt::Let {
                                span: *span,
                                pat: self.pattern(pat),
                                ty: ty.as_ref().map(|t| Box::new(self.ty(t))),
                                init: init.as_ref().map(|e| self.expr(e, nested)),
                            },
                            Stmt::Expr(e) => Stmt::Expr(self.expr(e, nested)),
                            Stmt::BlockLike(e) => Stmt::BlockLike(self.expr(e, nested)),
                            Stmt::Fn(f) => {
                                nested.push(f);
                                return None;
                            }
                            Stmt::Type(_) => return None,
                        })
                    })
                    .collect(),
                tail.as_ref().map(|t| Box::new(self.expr(t, nested))),
            ),
            ExprKind::StructLit(name, targs, inits) => ExprKind::StructLit(
                self.ident(name),
                self.types(targs),
                inits
                    .iter()
                    .map(|i| FieldInit {
                        name: i.name.clone(),
                        value: self.expr(&i.value, nested),
                    })
                    .collect(),
            ),
            ExprKind::Field(x, field) => {
                ExprKind::Field(Box::new(self.expr(x, nested)), field.clone())
            }
            ExprKind::Variant(ty, targs, variant, args) => ExprKind::Variant(
                self.ident(ty),
                self.types(targs),
                variant.clone(),
                args.iter().map(|a| self.expr(a, nested)).collect(),
            ),
            ExprKind::Match(scrutinee, arms) => ExprKind::Match(
                Box::new(self.expr(scrutinee, nested)),
                arms.iter()
                    .map(|a| Arm {
                        pat: self.pattern(&a.pat),
                        body: self.expr(&a.body, nested),
                    })
                    .collect(),
            ),
            // Its free variables change with the names in it.
            ExprKind::Lambda(l) => {
                let params = l
                    .params
                    .iter()
                    .map(|p| LambdaParam {
                        mutable: p.mutable,
                        name: p.name.clone(),
                        ty: p.ty.as_ref().map(|t| self.ty(t)),
                    })
                    .collect();
                let ret = l.ret.as_ref().map(|t| self.ty(t));
                let body = self.expr(&l.body, nested);
                ExprKind::Lambda(Arc::new(Lambda::new(params, ret, body)))
            }
            ExprKind::Assign(op, place, value) => ExprKind::Assign(
                *op,
                Box::new(self.expr(place, nested)),
                Box::new(self.expr(value, nested)),
            ),
            ExprKind::While(label, c, body) => ExprKind::While(
                label.clone(),
                Box::new(self.expr(c, nested)),
                Box::new(self.expr(body, nested)),
            ),
            ExprKind::Loop(label, body) => {
                ExprKind::Loop(label.clone(), Box::new(self.expr(body, nested)))
            }
            ExprKind::Return(x) => {
                ExprKind::Return(x.as_ref().map(|x| Box::new(self.expr(x, nested))))
            }
            ExprKind::Break(label, x) => ExprKind::Break(
                label.clone(),
                x.as_ref().map(|x| Box::new(self.expr(x, nested))),
            ),
        };
        let e = Expr { kind, span: e.span };
        match self.info.coercions.get(&e.span) {
            None => e,
            Some(&(derefs, mutable)) => {
                let span = e.span;
                let mut inner = e;
                for _ in 0..=derefs {
                    let kind = ExprKind::Unary(UnOp::Deref, Box::new(inner));
                    inner = Expr { kind, span };
                }
                let op = if mutable { UnOp::RefMut } else { UnOp::Ref };
                Expr {
                    kind: ExprKind::Unary(op, Box::new(inner)),
                    span,
                }
            }
        }
    }

    fn pattern(&self, p: &Pattern) -> Pattern {
        let kind = match &p.kind {
            PatternKind::Variant(ty, targs, variant, args) => PatternKind::Variant(
                self.ident(ty),
                self.types(targs),
                variant.clone(),
                args.iter().map(|a| self.pattern(a)).collect(),
            ),
            PatternKind::Struct(name, targs, fields, rest) => PatternKind::Struct(
                self.ident(name),
                self.types(targs),
                fields
                    .iter()
                    .map(|f| FieldPat {
                        name: f.name.clone(),
                        pat: self.pattern(&f.pat),
                    })
                    .collect(),
                *rest,
            ),
            PatternKind::Or(alts) => {
                PatternKind::Or(alts.iter().map(|a| self.pattern(a)).collect())
            }
            kind => kind.clone(),
        };
        Pattern { kind, span: p.span }
    }
}
