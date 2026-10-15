//! Lifting: moves every nested function to the top level, named by its
//! symbol (`main::helper`, see `check/scope.rs`), and renames every use of a
//! function to the symbol the checker found it refers to. A nested function
//! can't capture anything, so that's all it takes: it means the same at the
//! top level, under a name of its own. After this, nothing needs to know
//! there were nested functions, or that two functions can share a name:
//! not monomorphisation, not the interpreter, and not a backend.

use crate::ast::*;
use crate::check::Info;
use std::sync::Arc;

/// `prog`, which must have checked with `info`, with no nested functions.
/// Each one comes right after the function it was in, in order.
pub fn lift(prog: &Program, info: &Info) -> Program {
    let mut fns = Vec::new();
    for f in &prog.fns {
        lift_fn(f, info, &mut fns);
    }
    Program {
        types: prog.types.clone(),
        fns,
        stray_docs: prog.stray_docs.clone(),
    }
}

/// Adds `f`, named by its symbol, to `out`, then those nested in it.
fn lift_fn(f: &FnDecl, info: &Info, out: &mut Vec<FnDecl>) {
    let mut cx = Cx {
        info,
        nested: Vec::new(),
    };
    let body = cx.expr(&f.body);
    out.push(FnDecl {
        doc: f.doc.clone(),
        span: f.span,
        name: Ident {
            name: info.symbols[&f.name.span].clone(),
            span: f.name.span,
        },
        type_params: f.type_params.clone(),
        params: f.params.clone(),
        parens: f.parens,
        ret: f.ret.clone(),
        body,
    });
    for g in cx.nested {
        lift_fn(g, info, out);
    }
}

struct Cx<'p> {
    info: &'p Info,
    /// The functions nested in the one being lifted, in order.
    nested: Vec<&'p FnDecl>,
}

impl<'p> Cx<'p> {
    fn expr(&mut self, e: &'p Expr) -> Expr {
        let kind = match &e.kind {
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => e.kind.clone(),
            ExprKind::Var(name, targs) => {
                let name = self.info.symbols.get(&e.span).unwrap_or(name);
                ExprKind::Var(name.clone(), targs.clone())
            }
            ExprKind::Unary(op, x) => ExprKind::Unary(*op, Box::new(self.expr(x))),
            ExprKind::Binary(op, l, r) => {
                ExprKind::Binary(*op, Box::new(self.expr(l)), Box::new(self.expr(r)))
            }
            ExprKind::Call(f, args) => ExprKind::Call(
                Box::new(self.expr(f)),
                args.iter().map(|a| self.expr(a)).collect(),
            ),
            ExprKind::If(c, then, els) => ExprKind::If(
                Box::new(self.expr(c)),
                Box::new(self.expr(then)),
                els.as_ref().map(|e| Box::new(self.expr(e))),
            ),
            ExprKind::Block(stmts, tail) => ExprKind::Block(
                stmts
                    .iter()
                    .filter_map(|s| {
                        Some(match s {
                            Stmt::Let { pat, ty, init } => Stmt::Let {
                                pat: pat.clone(),
                                ty: ty.clone(),
                                init: self.expr(init),
                            },
                            Stmt::Expr(e) => Stmt::Expr(self.expr(e)),
                            Stmt::BlockLike(e) => Stmt::BlockLike(self.expr(e)),
                            Stmt::Fn(f) => {
                                self.nested.push(f);
                                return None;
                            }
                        })
                    })
                    .collect(),
                tail.as_ref().map(|t| Box::new(self.expr(t))),
            ),
            ExprKind::StructLit(name, targs, inits) => ExprKind::StructLit(
                name.clone(),
                targs.clone(),
                inits
                    .iter()
                    .map(|i| FieldInit {
                        name: i.name.clone(),
                        value: self.expr(&i.value),
                    })
                    .collect(),
            ),
            ExprKind::Field(x, field) => ExprKind::Field(Box::new(self.expr(x)), field.clone()),
            ExprKind::Variant(ty, targs, variant, args) => ExprKind::Variant(
                ty.clone(),
                targs.clone(),
                variant.clone(),
                args.iter().map(|a| self.expr(a)).collect(),
            ),
            ExprKind::Match(scrutinee, arms) => ExprKind::Match(
                Box::new(self.expr(scrutinee)),
                arms.iter()
                    .map(|a| Arm {
                        pat: a.pat.clone(),
                        body: self.expr(&a.body),
                    })
                    .collect(),
            ),
            // Its free variables change with the names in it.
            ExprKind::Lambda(l) => {
                let params = l
                    .params
                    .iter()
                    .map(|p| LambdaParam {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                    })
                    .collect();
                let body = self.expr(&l.body);
                ExprKind::Lambda(Arc::new(Lambda::new(params, l.ret.clone(), body)))
            }
        };
        Expr { kind, span: e.span }
    }
}
