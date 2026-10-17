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
//! A call of a trait's method, like `<Show>::show` at `Self = T`, becomes a
//! call of the impl's: once `T` is known, as it is in each copy, trait
//! resolution finds the one impl of `Show` for it (see `traits.rs`), and the
//! copy is of that impl's `show`, or the trait's default if it has none.
//! It's named after the type and trait, as `<Point as Show>::show`, and an
//! inherent method's after the type, as `<Point>::len`. Traits and impls
//! are gone after this: all that's left of them is plain functions, and
//! which types are `Copy` and `Eq`, which each type's copy says with a
//! `#[derive(..)]`. The `eq` of a built-in or derived `Eq` is `==`, which
//! the interpreter does field by field, calling `<T as Eq>::eq` for a type
//! `T` with an `eq` of its own.
//!
//! The interpreter runs what comes out of here, and `t mono` prints it.

use crate::ast::*;
use crate::check::{self, Info, Instance};
use crate::error::{Error, Span};
use crate::traits::Source;
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

/// A function to make copies of: its declaration, all its type parameters
/// (for a method, its impl's or `Self` first), what `Self` is in it, for a
/// method of an impl, and its impl's lifetime parameters.
struct Decl<'p> {
    f: &'p FnDecl,
    params: Vec<String>,
    self_ty: Option<Type>,
    lifetimes: Vec<Ident>,
}

/// What a call of a method names, once it's known what type it's of.
enum Target {
    /// Function `.0` at type arguments `.1`, whose copy is called `.2`.
    Fn(String, Vec<Type>, String),
    /// The `eq` of a built-in or derived `Eq`, of the type: `==`.
    Eq(Type),
}

struct Mono<'p> {
    prog: &'p Program,
    info: &'p Info,
    arities: HashMap<String, (usize, usize)>,
    fns: HashMap<&'p str, Decl<'p>>,
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
            fns: decls(prog, info),
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
                self.function(f, &Subst::default(), &[], None),
            );
        }
        let mut types: HashMap<String, TypeDecl> = HashMap::new();
        for d in prog.types.iter().filter(|d| d.type_params().is_empty()) {
            types.insert(d.name().name.clone(), self.type_decl(d, &[]));
        }
        loop {
            if let Some(name) = self.fn_queue.pop() {
                let copy = &self.copies[&name];
                let d = &self.fns[copy.of.as_str()];
                let (f, params) = (d.f, d.params.clone());
                let lifetimes = d.lifetimes.clone();
                let mut s = Subst::new(&params, &copy.args.clone());
                if let Some(t) = &d.self_ty {
                    let t = s.apply(t);
                    s = s.with_type("Self", t);
                }
                let mut decl = self.function(f, &s, &params, Some(&name));
                decl.generics.lifetimes.splice(0..0, lifetimes);
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
            traits: Vec::new(),
            impls: Vec::new(),
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
        // Then the methods, in the order they were first needed.
        for copy in &self.copy_order {
            out.fns.extend(fns.remove(copy));
        }
        out
    }

    /// The name of the copy of function `name` for `args`, `mangled`,
    /// making it if it's new. `at` is the use that needs it, in copy `by`,
    /// if any.
    fn named_copy(
        &mut self,
        name: &str,
        args: Vec<Type>,
        mangled: String,
        at: Span,
        by: Option<&str>,
    ) -> String {
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

    /// The copy of `f`, which has type parameters `params`, with them
    /// replaced as `s` says, named `name` (or as it was, if it isn't
    /// generic).
    fn function(&mut self, f: &FnDecl, s: &Subst, params: &[String], name: Option<&str>) -> FnDecl {
        let mut params = params.to_vec();
        params.push("Self".into());
        let mut cx = Cx {
            m: self,
            s,
            params,
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
        // Whether it's `Copy` and `Eq`, which the checker of what this
        // makes takes the copy's word for.
        let ty = Type::Adt(d.name().name.clone(), Vec::new(), args.to_vec());
        let traits = &cx.m.info.traits;
        let names: Vec<Ident> = ["Copy", "Eq"]
            .into_iter()
            .filter(|tr| traits.implements(&ty, tr, &[]).is_ok())
            .map(|tr| Ident {
                name: tr.into(),
                span: d.name().span,
            })
            .collect();
        let derives = match names.is_empty() {
            true => Vec::new(),
            false => vec![Derive {
                span: d.name().span,
                names,
            }],
        };
        match d {
            TypeDecl::Struct(st) => TypeDecl::Struct(StructDecl {
                doc: st.doc.clone(),
                allows: st.allows.clone(),
                span: st.span,
                name,
                generics: lifetimes_only(&st.generics),
                derives,
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
                derives,
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
        let decl = &self.fns[copy.of.as_str()];
        let bindings: Vec<String> = decl
            .params
            .iter()
            .zip(&copy.args)
            .map(|(p, a)| format!("`{p}` = `{a}`"))
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

/// Every function of `prog`, which checked with `info` and was lifted, by
/// its name: its symbol. Methods too, those of impls and traits' defaults.
fn decls<'p>(prog: &'p Program, info: &Info) -> HashMap<&'p str, Decl<'p>> {
    let mut out: HashMap<&str, Decl> = prog
        .fns
        .iter()
        .map(|f| {
            let d = Decl {
                f,
                params: names(&f.generics.types),
                self_ty: None,
                lifetimes: Vec::new(),
            };
            (f.name.name.as_str(), d)
        })
        .collect();
    for d in &prog.impls {
        for f in &d.methods {
            let symbol = f.name.name.as_str();
            let def = info
                .traits
                .impls
                .iter()
                .find(|i| i.methods.iter().any(|(_, s)| s == symbol))
                .expect("a method of an impl");
            let mut params = def.params.clone();
            params.extend(names(&f.generics.types));
            let decl = Decl {
                f,
                params,
                self_ty: Some(def.self_ty.clone()),
                lifetimes: d.generics.lifetimes.clone(),
            };
            out.insert(symbol, decl);
        }
    }
    for t in &prog.traits {
        for m in t.methods.iter().filter(|m| m.provided) {
            let mut params = vec!["Self".to_string()];
            params.extend(names(&m.decl.generics.types));
            let decl = Decl {
                f: &m.decl,
                params,
                self_ty: None,
                lifetimes: Vec::new(),
            };
            out.insert(m.decl.name.name.as_str(), decl);
        }
    }
    out
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
            Type::Never => TypeExprKind::Never,
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
                match self.target(&name, args) {
                    Target::Fn(of, args, copy) => {
                        self.m.named_copy(&of, args, copy, span, self.copy)
                    }
                    Target::Eq(_) => unreachable!("`eq` of a built-in `Eq` is `==`"),
                }
            }
            Instance::Adt(name, args) => {
                let args: Vec<Type> = args.iter().map(|a| self.s.apply(a).erased()).collect();
                // Its fields' types may need copies of their own.
                self.written(&Type::Adt(name.clone(), Vec::new(), args.clone()), span);
                self.m.type_copy(&name, args)
            }
        })
    }

    /// What function `name`, at type arguments `args` with no type
    /// parameters left in them, is: itself, unless it's a trait's method,
    /// which is the method of the impl for its `Self`. Named for its copy.
    fn target(&mut self, name: &str, args: Vec<Type>) -> Target {
        let traits = &self.m.info.traits;
        let of_trait = traits
            .traits
            .iter()
            .find_map(|(t, d)| Some((t.clone(), d.methods.iter().find(|m| m.symbol == name)?)));
        let impl_of = || {
            let d = traits
                .impls
                .iter()
                .find(|d| d.methods.iter().any(|(_, s)| s == name))?;
            let m = d.methods.iter().find(|(_, s)| s == name)?.0.clone();
            Some((d.clone(), m))
        };
        let (tr, method, self_ty, own) = match (of_trait, impl_of()) {
            (Some((tr, m)), _) => {
                let (self_ty, own) = args.split_first().expect("`Self` first");
                (Some(tr), m.name.clone(), self_ty.clone(), own.to_vec())
            }
            (None, Some((d, m))) => {
                let (of_impl, own) = args.split_at(d.params.len());
                let self_ty = Subst::new(&d.params, of_impl).apply(&d.self_ty);
                // A method of an impl of a trait is only called through
                // the trait's, so this is an inherent one.
                (None, m, self_ty, own.to_vec())
            }
            (None, None) => {
                let copy = mangle(name, &args);
                return Target::Fn(name.to_string(), args, copy);
            }
        };
        let header = match &tr {
            Some(tr) => format!("{} as {tr}", self.copy_name(&self_ty)),
            None => self.copy_name(&self_ty),
        };
        let copy = mangle(&format!("<{header}>::{method}"), &own);
        let Some(tr) = tr else {
            return Target::Fn(name.to_string(), args, copy);
        };
        let traits = &self.m.info.traits;
        match traits.implements(&self_ty, &tr, &[]) {
            Ok(Source::Impl(i, impl_args)) if !traits.impls[i].derived => {
                let d = &traits.impls[i];
                match d.methods.iter().find(|(m, _)| *m == method) {
                    Some((_, symbol)) => {
                        let mut args = impl_args;
                        args.extend(own);
                        Target::Fn(symbol.clone(), args, copy)
                    }
                    // The trait's default.
                    None => Target::Fn(name.to_string(), args, copy),
                }
            }
            Ok(Source::Impl(..) | Source::Builtin) => Target::Eq(self_ty),
            r => unreachable!("`{self_ty}: {tr}` in a copy: {r:?}"),
        }
    }

    /// How type `t`, with no type parameters left in it, is named in the
    /// name of a copy of a method: with the names of copies of structs and
    /// enums, as values of them are, and no lifetimes.
    fn copy_name(&mut self, t: &Type) -> String {
        match t {
            Type::Adt(name, _, args) => {
                let args: Vec<Type> = args.iter().map(Type::erased).collect();
                self.written(t, Span::new(0, 0));
                mangle(name, &args)
            }
            Type::Ref(_, m, inner) => {
                format!("&{}{}", if *m { "mut " } else { "" }, self.copy_name(inner))
            }
            Type::Box(inner) => format!("Box<{}>", self.copy_name(inner)),
            Type::Fn(sig) => {
                let params: Vec<String> = sig.params.iter().map(|p| self.copy_name(p)).collect();
                format!("fn({}) -> {}", params.join(", "), self.copy_name(&sig.ret))
            }
            t => t.to_string(),
        }
    }

    /// `|a: &T, b: &T| *a == *b`, at `span`: the `eq` of a built-in or
    /// derived `Eq` of `T`, as a function.
    fn eq_fn(&mut self, t: &Type, span: Span) -> ExprKind {
        let r = Type::Ref(Region::Erased, false, Box::new(t.clone()));
        let var = |name: &str| Expr {
            kind: ExprKind::Var(name.into(), Vec::new()),
            span,
        };
        let deref = |e: Expr| Expr {
            kind: ExprKind::Unary(UnOp::Deref, Box::new(e)),
            span,
        };
        let params = ["a", "b"]
            .map(|name| LambdaParam {
                mutable: None,
                name: Ident {
                    name: name.into(),
                    span,
                },
                ty: Some(self.written(&r, span)),
            })
            .into();
        let body = Expr {
            kind: ExprKind::Binary(
                BinOp::Eq,
                Box::new(deref(var("a"))),
                Box::new(deref(var("b"))),
            ),
            span,
        };
        ExprKind::Lambda(Arc::new(Lambda::new(params, None, body)))
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
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit | ExprKind::Continue(_) => {
                e.kind.clone()
            }
            // Type arguments written in the source are in `info.instances`
            // like the inferred ones, and the copy's name says them.
            ExprKind::Var(name, _) => match self.m.info.instances.get(&e.span).cloned() {
                Some(Instance::Fn(f, args)) => {
                    let args = args.iter().map(|a| self.s.apply(a).erased()).collect();
                    match self.target(&f, args) {
                        Target::Fn(of, args, copy) => {
                            let name = self.m.named_copy(&of, args, copy, e.span, self.copy);
                            ExprKind::Var(name, Vec::new())
                        }
                        Target::Eq(t) => self.eq_fn(&t, e.span),
                    }
                }
                // A method that isn't generic still needs its copy made.
                None if self
                    .m
                    .fns
                    .get(name.as_str())
                    .is_some_and(|d| d.self_ty.is_some()) =>
                {
                    match self.target(name, Vec::new()) {
                        Target::Fn(of, args, copy) => {
                            let name = self.m.named_copy(&of, args, copy, e.span, self.copy);
                            ExprKind::Var(name, Vec::new())
                        }
                        Target::Eq(_) => unreachable!("an inherent method"),
                    }
                }
                _ => ExprKind::Var(
                    self.instance(e.span).unwrap_or_else(|| name.clone()),
                    Vec::new(),
                ),
            },
            ExprKind::Unary(op, x) => ExprKind::Unary(*op, Box::new(self.expr(x))),
            ExprKind::Binary(op, l, r) => {
                ExprKind::Binary(*op, Box::new(self.expr(l)), Box::new(self.expr(r)))
            }
            ExprKind::Call(f, args) => ExprKind::Call(
                Box::new(self.expr(f)),
                args.iter().map(|a| self.expr(a)).collect(),
            ),
            ExprKind::MethodCall(..) => unreachable!("lifting makes method calls plain calls"),
            ExprKind::If(c, then, els) => ExprKind::If(
                Box::new(self.expr(c)),
                Box::new(self.expr(then)),
                els.as_ref().map(|e| Box::new(self.expr(e))),
            ),
            ExprKind::Block(stmts, tail) => ExprKind::Block(
                stmts
                    .iter()
                    .map(|s| match s {
                        Stmt::Let {
                            pat,
                            ty,
                            init,
                            span,
                        } => Stmt::Let {
                            span: *span,
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
            ExprKind::Variant(ty, _, variant, args, parens) => ExprKind::Variant(
                self.ident(ty),
                Vec::new(),
                variant.clone(),
                args.iter().map(|a| self.expr(a)).collect(),
                *parens,
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
            ExprKind::While(label, c, body) => ExprKind::While(
                label.clone(),
                Box::new(self.expr(c)),
                Box::new(self.expr(body)),
            ),
            ExprKind::Loop(label, body) => ExprKind::Loop(label.clone(), Box::new(self.expr(body))),
            ExprKind::Return(x) => ExprKind::Return(x.as_ref().map(|x| Box::new(self.expr(x)))),
            ExprKind::Break(label, x) => {
                ExprKind::Break(label.clone(), x.as_ref().map(|x| Box::new(self.expr(x))))
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
