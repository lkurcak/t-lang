//! Monomorphisation: turns a checked program with generics into one
//! without, by making a copy of each generic function, struct and enum for
//! each list of type arguments it's used with. `id(1)` and `id(true)` call
//! two different functions, `id::<int>` and `id::<bool>`, each of which
//! is plain non-generic code. See docs/04-generics.md.
//!
//! The copies are named after what they're copies of, like `id::<int>` and
//! `Option::<int>`. No name in the source can look like that, so they can't
//! clash with anything.
//!
//! The checker has already worked out the type arguments of every use of
//! something generic (`Info::instances`). In a generic function those can
//! mention its own type parameters; the copy for `T = int` substitutes
//! `int` for them, and so on down. It finishes because the checker rejects
//! polymorphic recursion.
//!
//! It works on a program with no nested functions or local types left (see
//! `lift.rs`), so everything has a name of its own.
//!
//! The interpreter runs what comes out of here, and `t mono` prints it.

use crate::ast::*;
use crate::check::{self, Info, Instance};
use crate::error::{Error, Span};
use crate::types::{Region, Subst, Type};
use std::collections::HashMap;
use std::sync::Arc;

/// `prog`, which must have checked with `info` and then been lifted,
/// without generics.
///
/// What comes out is checked again. That catches one kind of error the
/// checker can't see in a generic function's body, where it only knows `T`
/// as `T`: a closure that captures, escaping the frame that made it (see
/// `check/escape.rs`). `fn id<T>(x: T) -> T { x }` is fine, but its copy
/// for a function type returns a function parameter, which may be such a
/// closure. That's reported at the use that made the copy.
pub fn monomorphise(prog: &Program, info: &Info) -> Result<Program, Error> {
    let mut m = Mono::new(prog, info);
    let out = m.run();
    match check::recheck(&out) {
        Ok(()) => Ok(out),
        Err((Some(f), e)) if m.copies.contains_key(&f) => Err(m.blame(&f, e)),
        Err((_, e)) => Err(Error::ty(
            e.span,
            format!(
                "internal error: monomorphisation made a program that doesn't check: {}. \
                 This is a bug in T.",
                e.message
            ),
        )),
    }
}

/// The name of the copy of `name` for type arguments `args`. It has the
/// symbols of the types in them, which tell apart types of the same name,
/// where `Type`'s `Display` shows the names as written.
fn mangle(name: &str, args: &[Type]) -> String {
    match args {
        [] => name.to_string(),
        args => {
            let args: Vec<String> = args.iter().map(Type::symbols).collect();
            format!("{name}::<{}>", args.join(", "))
        }
    }
}

/// A copy of a generic function: what it's a copy of, and which use first
/// needed it, for blaming that use if the copy doesn't check.
struct Copy {
    of: String,
    args: Vec<Type>,
    /// The span of that use, and the copy it's in, if it's in one.
    needed_at: Span,
    needed_by: Option<String>,
}

struct Mono<'p> {
    prog: &'p Program,
    info: &'p Info,
    arities: HashMap<String, (usize, usize)>,
    fns: HashMap<&'p str, &'p FnDecl>,
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
}

impl<'p> Mono<'p> {
    fn new(prog: &'p Program, info: &'p Info) -> Mono<'p> {
        Mono {
            prog,
            info,
            arities: prog
                .types
                .iter()
                .map(|d| {
                    let arity = (d.generics().lifetimes.len(), d.type_params().len());
                    (d.name().name.clone(), arity)
                })
                .collect(),
            fns: prog.fns.iter().map(|f| (f.name.name.as_str(), f)).collect(),
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
                self.function(f, &Subst::default(), None),
            );
        }
        let mut types: HashMap<String, TypeDecl> = HashMap::new();
        for d in prog.types.iter().filter(|d| d.type_params().is_empty()) {
            types.insert(d.name().name.clone(), self.type_decl(d, &[]));
        }
        loop {
            if let Some(name) = self.fn_queue.pop() {
                let copy = &self.copies[&name];
                let f = self.fns[copy.of.as_str()];
                let s = Subst::new(&names(&f.generics.types), &copy.args.clone());
                let decl = self.function(f, &s, Some(&name));
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
            types: Vec::new(),
            fns: Vec::new(),
            stray_docs: Vec::new(),
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
        out
    }

    /// The name of the copy of function `name` for `args`, making it if
    /// it's new. `at` is the use that needs it, in copy `by`, if any.
    fn fn_copy(&mut self, name: &str, args: Vec<Type>, at: Span, by: Option<&str>) -> String {
        let mangled = mangle(name, &args);
        if !self.copies.contains_key(&mangled) {
            self.fn_queue.push(mangled.clone());
            let copy = Copy {
                of: name.to_string(),
                args,
                needed_at: at,
                needed_by: by.map(str::to_string),
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

    /// The copy of `f` with type parameters replaced as `s` says, named
    /// `name` (or as it was, if it isn't generic).
    fn function(&mut self, f: &FnDecl, s: &Subst, name: Option<&str>) -> FnDecl {
        let mut cx = Cx {
            m: self,
            s,
            params: names(&f.generics.types),
            copy: name,
        };
        FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            name: Ident {
                name: name.unwrap_or(&f.name.name).to_string(),
                span: f.name.span,
            },
            generics: lifetimes_only(&f.generics),
            params: f
                .params
                .iter()
                .map(|p| Param {
                    doc: None,
                    allows: Vec::new(),
                    mutable: p.mutable,
                    name: p.name.clone(),
                    ty: cx.ty(&p.ty),
                })
                .collect(),
            parens: f.parens,
            ret: cx.ty(&f.ret),
            body: cx.expr(&f.body),
        }
    }

    /// The copy of struct or enum `d` for type arguments `args`.
    fn type_decl(&mut self, d: &TypeDecl, args: &[Type]) -> TypeDecl {
        let params = names(d.type_params());
        let s = Subst::new(&params, args);
        let mut cx = Cx {
            m: self,
            s: &s,
            params,
            copy: None,
        };
        let name = Ident {
            name: mangle(&d.name().name, args),
            span: d.name().span,
        };
        match d {
            TypeDecl::Struct(st) => TypeDecl::Struct(StructDecl {
                doc: st.doc.clone(),
                allows: st.allows.clone(),
                span: st.span,
                name,
                generics: lifetimes_only(&st.generics),
                derives: st.derives.clone(),
                fields: st
                    .fields
                    .iter()
                    .map(|f| Param {
                        doc: f.doc.clone(),
                        allows: f.allows.clone(),
                        mutable: None,
                        name: f.name.clone(),
                        ty: cx.ty(&f.ty),
                    })
                    .collect(),
            }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl {
                doc: e.doc.clone(),
                allows: e.allows.clone(),
                span: e.span,
                name,
                generics: lifetimes_only(&e.generics),
                derives: e.derives.clone(),
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
        }
    }

    /// The error for copy `f` not checking: `e`, at the use outside any
    /// generic function that it comes from. It names functions as they're
    /// written, not by their symbols.
    fn blame(&self, f: &str, e: Error) -> Error {
        // Up through the copies that needed it.
        let mut top = f;
        while let Some(by) = &self.copies[top].needed_by {
            top = by;
        }
        let copy = &self.copies[top];
        let decl = self.fns[copy.of.as_str()];
        let bindings: Vec<String> = decl
            .generics
            .types
            .iter()
            .zip(&copy.args)
            .map(|(p, a)| format!("`{}` = `{a}`", p.name))
            .collect();
        let inside = match top == f {
            true => "in that copy of it".to_string(),
            false => format!("in `{}`, which that needs", check::source_name(f)),
        };
        Error::ty(
            copy.needed_at,
            format!(
                "cannot use `{}` with {} here: {inside}, {}",
                check::source_name(&copy.of),
                bindings.join(", "),
                e.message
            ),
        )
    }
}

fn names(params: &[Ident]) -> Vec<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

/// `g` without its type parameters, which a copy doesn't have, and the
/// bounds on them. Copies are generic over lifetimes as before.
fn lifetimes_only(g: &Generics) -> Generics {
    Generics {
        lifetimes: g.lifetimes.clone(),
        types: Vec::new(),
        bounds: g
            .bounds
            .iter()
            .filter(|(p, _)| p.name.starts_with('\''))
            .cloned()
            .collect(),
        span: g.span,
    }
}

/// Copying one function or type.
struct Cx<'m, 'p, 's> {
    m: &'m mut Mono<'p>,
    /// What each type parameter of the one being copied stands for.
    s: &'s Subst,
    /// Its type parameters.
    params: Vec<String>,
    /// The name of the copy, if it's a copy of a generic function.
    copy: Option<&'s str>,
}

impl Cx<'_, '_, '_> {
    /// A type written in the original, as written in the copy.
    fn ty(&mut self, t: &TypeExpr) -> TypeExpr {
        // Everything is at the top level by now, named by its symbol.
        let arities = &self.m.arities;
        let names = |name: &str, _| match self.params.iter().any(|p| p == name) {
            true => None,
            false => arities
                .get(name)
                .map(|&(l, n)| check::TypeName::Adt(name.to_string(), l, n)),
        };
        let resolved = check::resolve(t, &names, &self.params, &mut check::Lifetimes::Any)
            .expect("monomorphising a program that checked");
        self.written(&self.s.apply(&resolved), t.span)
    }

    /// How to write `t`, which has no type parameters left in it.
    fn written(&mut self, t: &Type, span: Span) -> TypeExpr {
        let kind = match t {
            Type::Int => TypeExprKind::Named("int".into(), Vec::new()),
            Type::Bool => TypeExprKind::Named("bool".into(), Vec::new()),
            Type::Unit => TypeExprKind::Unit,
            Type::Fn(sig) => TypeExprKind::Fn(
                sig.params.iter().map(|p| self.written(p, span)).collect(),
                Box::new(self.written(&sig.ret, span)),
            ),
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
                    }),
                };
                TypeExprKind::Ref(lifetime, *m, Box::new(self.written(inner, span)))
            }
            Type::Box(inner) => TypeExprKind::Named("Box".into(), vec![self.written(inner, span)]),
            Type::Param(_) | Type::Var(_) => unreachable!("`{t}` left after monomorphising"),
        };
        TypeExpr { kind, span }
    }

    /// The use of something generic at `span`, if that's what's there, as
    /// the name of its copy.
    fn instance(&mut self, span: Span) -> Option<String> {
        let instance = self.m.info.instances.get(&span)?.clone();
        Some(match instance {
            // A copy doesn't depend on lifetimes, so they're left out.
            Instance::Fn(name, args) => {
                let args = args.iter().map(|a| self.s.apply(a).erased()).collect();
                self.m.fn_copy(&name, args, span, self.copy)
            }
            Instance::Adt(name, args) => {
                let args: Vec<Type> = args.iter().map(|a| self.s.apply(a).erased()).collect();
                // Its fields' types may need copies of their own.
                self.written(&Type::Adt(name.clone(), Vec::new(), args.clone()), span);
                self.m.type_copy(&name, args)
            }
        })
    }

    /// `id`, renamed to its copy if it names something generic.
    fn ident(&mut self, id: &Ident) -> Ident {
        Ident {
            name: self.instance(id.span).unwrap_or_else(|| id.name.clone()),
            span: id.span,
        }
    }

    fn expr(&mut self, e: &Expr) -> Expr {
        let kind = match &e.kind {
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => e.kind.clone(),
            // Type arguments written in the source are in `info.instances`
            // like the inferred ones, and the copy's name says them.
            ExprKind::Var(name, _) => ExprKind::Var(
                self.instance(e.span).unwrap_or_else(|| name.clone()),
                Vec::new(),
            ),
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
                    .map(|s| match s {
                        Stmt::Let { pat, ty, init } => Stmt::Let {
                            pat: self.pattern(pat),
                            ty: ty.as_ref().map(|t| Box::new(self.ty(t))),
                            init: init.as_ref().map(|e| self.expr(e)),
                        },
                        Stmt::Expr(e) => Stmt::Expr(self.expr(e)),
                        Stmt::BlockLike(e) => Stmt::BlockLike(self.expr(e)),
                        Stmt::Fn(_) | Stmt::Type(_) => {
                            unreachable!("nested functions and types are lifted first")
                        }
                    })
                    .collect(),
                tail.as_ref().map(|t| Box::new(self.expr(t))),
            ),
            ExprKind::StructLit(name, _, inits) => ExprKind::StructLit(
                self.ident(name),
                Vec::new(),
                inits
                    .iter()
                    .map(|i| FieldInit {
                        name: i.name.clone(),
                        value: self.expr(&i.value),
                    })
                    .collect(),
            ),
            ExprKind::Field(x, field) => ExprKind::Field(Box::new(self.expr(x)), field.clone()),
            ExprKind::Variant(ty, _, variant, args) => ExprKind::Variant(
                self.ident(ty),
                Vec::new(),
                variant.clone(),
                args.iter().map(|a| self.expr(a)).collect(),
            ),
            ExprKind::Match(scrutinee, arms) => ExprKind::Match(
                Box::new(self.expr(scrutinee)),
                arms.iter()
                    .map(|a| Arm {
                        pat: self.pattern(&a.pat),
                        body: self.expr(&a.body),
                    })
                    .collect(),
            ),
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
                ExprKind::Lambda(Arc::new(Lambda::new(params, ret, self.expr(&l.body))))
            }
            ExprKind::Assign(op, place, value) => {
                ExprKind::Assign(*op, Box::new(self.expr(place)), Box::new(self.expr(value)))
            }
            ExprKind::While(c, body) => {
                ExprKind::While(Box::new(self.expr(c)), Box::new(self.expr(body)))
            }
        };
        Expr { kind, span: e.span }
    }

    fn pattern(&mut self, p: &Pattern) -> Pattern {
        let kind = match &p.kind {
            PatternKind::Variant(ty, _, variant, args) => PatternKind::Variant(
                self.ident(ty),
                Vec::new(),
                variant.clone(),
                args.iter().map(|a| self.pattern(a)).collect(),
            ),
            PatternKind::Struct(name, _, fields, rest) => PatternKind::Struct(
                self.ident(name),
                Vec::new(),
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

/// How region `r` is written: `'a`, or `'_` for one nobody named.
fn region_name(r: &Region) -> String {
    match r {
        Region::Named(n) if !r.is_elided() && !n.starts_with("'^") => n.clone(),
        Region::Static => "'static".into(),
        _ => "'_".into(),
    }
}
