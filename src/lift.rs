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
//! And method calls and paths become plain calls of the methods, named by
//! their symbols, with the receiver as the first argument, as the method
//! takes it: `v.push(1)` becomes `<V>::push(&mut v, 1)`. The methods stay
//! in their impls and traits, renamed the same way, for monomorphisation
//! to pick from.
//!
//! And it writes out what the borrow checker moved, rather than copied
//! (`Info::moves`): `ExprKind::Move` and `PatternKind::Move`. That's
//! decided by the types the checker knew, so a generic body moves what it
//! moved when it was checked, whatever its type parameters turn out to be.

use crate::ast::*;
use crate::check::{Info, MoveSite};
use crate::traits::FnKind;
use std::cell::Cell;
use std::sync::Arc;

/// `prog`, which must have checked with `info`, with no nested functions or
/// local types. Each function nested in another comes right after it, in
/// order, and the local types come after the top-level ones, in the order of
/// `Program::all_types`.
pub fn lift(prog: &Program, info: &Info) -> Program {
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
        alloc: None,
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
    }
}

struct Cx<'p> {
    info: &'p Info,
    /// `Program::next_id`: the nodes written here that weren't in the
    /// program take ids from it. The others keep theirs.
    next_id: Cell<u32>,
}

impl<'p> Cx<'p> {
    /// An id for a node written here (see `NodeId`).
    fn id(&self) -> NodeId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        NodeId(id)
    }

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
        let body = self.expr(&f.body, &mut nested);
        let decl = FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            module: ROOT,
            public: f.public,
            name: self.ident(&f.name),
            generics: self.generics(&f.generics),
            params: f.params.iter().map(|p| self.param(p)).collect(),
            parens: f.parens,
            ret: self.ty(&f.ret),
            effects: f.effects.as_ref().map(|e| self.effects(e)),
            body,
            intrinsic: f.intrinsic,
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
                public: s.public,
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
                public: e.public,
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
            public: p.public,
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

    /// `e`, renamed, adding the functions nested in it to `nested`, in
    /// order. Local types are taken out too: `lift` has them already.
    fn expr(&self, e: &'p Expr, nested: &mut Vec<&'p FnDecl>) -> Expr {
        let kind = match &e.kind {
            ExprKind::For(_) | ExprKind::Range(..) => {
                unreachable!("rewritten when paths are resolved")
            }
            ExprKind::Move(_) => unreachable!("written here"),
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit | ExprKind::Continue(_) => {
                e.kind.clone()
            }
            ExprKind::Var(name, targs) => {
                ExprKind::Var(self.renamed(name, e.id), self.types(targs))
            }
            ExprKind::Unary(op, x) => ExprKind::Unary(*op, Box::new(self.expr(x, nested))),
            ExprKind::Binary(op, l, r) => ExprKind::Binary(
                *op,
                Box::new(self.expr(l, nested)),
                Box::new(self.expr(r, nested)),
            ),
            // A call of what isn't a function: of what it is through
            // `derefs` references, by `&` or `&mut`, or by value. One of a
            // borrow, `(&f)(1)`, is that already.
            ExprKind::Call(f, args) => {
                let mut callee = self.expr(f, nested);
                let borrow = matches!(f.kind, ExprKind::Unary(UnOp::Ref | UnOp::RefMut, _));
                if let Some(&(derefs, kind)) = self.info.calls.get(&e.id)
                    && !borrow
                {
                    let span = callee.span;
                    for _ in 0..derefs {
                        let kind = ExprKind::Unary(UnOp::Deref, Box::new(callee));
                        callee = Expr {
                            kind,
                            span,
                            id: self.id(),
                        };
                    }
                    let op = match kind {
                        FnKind::Fn => Some(UnOp::Ref),
                        FnKind::FnMut => Some(UnOp::RefMut),
                        FnKind::FnOnce => None,
                    };
                    if let Some(op) = op {
                        let kind = ExprKind::Unary(op, Box::new(callee));
                        callee = Expr {
                            kind,
                            span,
                            id: self.id(),
                        };
                    } else if self.info.moves.contains(&MoveSite::Receiver(e.id)) {
                        let kind = ExprKind::Move(Box::new(callee));
                        callee = Expr {
                            kind,
                            span,
                            id: self.id(),
                        };
                    }
                }
                ExprKind::Call(
                    Box::new(callee),
                    args.iter().map(|a| self.expr(a, nested)).collect(),
                )
            }
            ExprKind::MethodCall(x, name, _, args) => {
                let m = &self.info.methods[&name.id];
                let mut recv = self.expr(x, nested);
                let span = recv.span;
                for _ in 0..m.derefs {
                    let kind = ExprKind::Unary(UnOp::Deref, Box::new(recv));
                    recv = Expr {
                        kind,
                        span,
                        id: self.id(),
                    };
                }
                if let Some(mutable) = m.autoref {
                    let op = if mutable { UnOp::RefMut } else { UnOp::Ref };
                    let kind = ExprKind::Unary(op, Box::new(recv));
                    recv = Expr {
                        kind,
                        span,
                        id: self.id(),
                    };
                } else if self.info.moves.contains(&MoveSite::Receiver(e.id)) {
                    let kind = ExprKind::Move(Box::new(recv));
                    recv = Expr {
                        kind,
                        span,
                        id: self.id(),
                    };
                }
                // As the method's name: what the checker noted of it is
                // what it noted of the name.
                let f = Expr {
                    kind: ExprKind::Var(m.symbol.clone(), Vec::new()),
                    span: name.span,
                    id: name.id,
                };
                let args = std::iter::once(recv)
                    .chain(args.iter().map(|a| self.expr(a, nested)))
                    .collect();
                ExprKind::Call(Box::new(f), args)
            }
            ExprKind::Variant(_, _, name, args, parens)
                if self.info.methods.contains_key(&name.id) =>
            {
                let f = ExprKind::Var(self.info.methods[&name.id].symbol.clone(), Vec::new());
                let f = Expr {
                    kind: f,
                    span: name.span,
                    id: name.id,
                };
                match parens {
                    true => {
                        let args = args.iter().map(|a| self.expr(a, nested)).collect();
                        ExprKind::Call(Box::new(f), args)
                    }
                    // Named by the whole of it, as the checker noted.
                    false => f.kind,
                }
            }
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
            ExprKind::Variant(ty, targs, variant, args, parens) => ExprKind::Variant(
                self.ident(ty),
                self.types(targs),
                variant.clone(),
                args.iter().map(|a| self.expr(a, nested)).collect(),
                *parens,
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
                let mut lambda = Lambda::new(l.id, params, ret, body);
                lambda.moves = l.moves;
                let captures = self.info.captures.get(&l.id).into_iter().flatten();
                lambda.modes = (captures.enumerate())
                    .map(|(i, (name, mode))| {
                        let site = MoveSite::Capture(l.id, i);
                        (name.clone(), *mode, self.info.moves.contains(&site))
                    })
                    .collect();
                ExprKind::Lambda(Arc::new(lambda))
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
            ExprKind::Unsafe(block) => ExprKind::Unsafe(Box::new(self.expr(block, nested))),
            ExprKind::Return(x) => {
                ExprKind::Return(x.as_ref().map(|x| Box::new(self.expr(x, nested))))
            }
            ExprKind::Break(label, x) => ExprKind::Break(
                label.clone(),
                x.as_ref().map(|x| Box::new(self.expr(x, nested))),
            ),
        };
        let moves = self.info.moves.contains(&MoveSite::Expr(e.id));
        debug_assert!(!moves || e.is_place(), "only a place is moved out of");
        let e = Expr {
            kind,
            span: e.span,
            id: e.id,
        };
        if moves {
            let span = e.span;
            return Expr {
                kind: ExprKind::Move(Box::new(e)),
                span,
                id: self.id(),
            };
        }
        match self.info.coercions.get(&e.id) {
            None => e,
            Some(&(derefs, mutable)) => {
                let span = e.span;
                let mut inner = e;
                for _ in 0..=derefs {
                    let kind = ExprKind::Unary(UnOp::Deref, Box::new(inner));
                    inner = Expr {
                        kind,
                        span,
                        id: self.id(),
                    };
                }
                let op = if mutable { UnOp::RefMut } else { UnOp::Ref };
                Expr {
                    kind: ExprKind::Unary(op, Box::new(inner)),
                    span,
                    id: self.id(),
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
            PatternKind::Move(_) => unreachable!("written here"),
            kind => kind.clone(),
        };
        let pat = Pattern {
            kind,
            span: p.span,
            id: p.id,
        };
        match self.info.moves.contains(&MoveSite::Pattern(p.id)) {
            true => Pattern {
                kind: PatternKind::Move(Box::new(pat)),
                span: p.span,
                id: self.id(),
            },
            false => pat,
        }
    }
}
