//! Monomorphisation: turns a checked program with generics into one
//! without, by making a copy of each generic function, struct and enum for
//! each list of type arguments it's used with. `id(1)` and `id(true)` call
//! two different functions, `id::<i64>` and `id::<bool>`, each of which
//! is plain non-generic code. See docs/04-generics.md.
//!
//! The copies are named after what they're copies of, like `id::<i64>` and
//! `Option::<i64>`. No name in the source can look like that, so they can't
//! clash with anything.
//!
//! The checker has already worked out the type arguments of every use of
//! something generic (`Info::instances`). In a generic function those can
//! mention its own type parameters; the copy for `T = i64` substitutes
//! `i64` for them, and so on down. It finishes because the checker rejects
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
//! `T` with an `eq` of its own. So each `==` needs a copy of every such
//! `eq` it may get to, through fields and what references and boxes point
//! to (`Cx::need_eq`): the interpreter can't tell a missing one from a
//! derived one.
//!
//! An opaque type, `impl Fn(i64) -> i64` in a function's return type, is
//! replaced by the type it stands for, in the copy at the type arguments
//! it's at (`Projections::reveal`). That's sound for the same reason as for
//! a trait's method: the checker only let callers rely on its bounds, which
//! that type implements, by the impls it'll find here.
//!
//! The interpreter runs what comes out of here, and `t mono` prints it.

use crate::ast::*;
use crate::check::{self, Info, Instance};
use crate::error::Span;
use crate::traits::{Projections, Source, eq_subject};
use crate::types::{Params, Region, Subst, Type};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// `prog`, which must have checked with `info` and then been lifted,
/// without generics.
///
/// Nothing a copy does can be wrong where the generic function wasn't: the
/// checker checked it for every type its type parameters could be, and
/// what a copy does depends on its types only through the impls the bounds
/// said there were.
pub fn monomorphise(prog: &Program, info: &Info) -> Program {
    Mono::new(prog, info).run()
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

/// A copy of a generic function: what it's a copy of, at which type
/// arguments.
struct Copy {
    of: String,
    args: Vec<Type>,
}

/// A function to make copies of: its declaration, all its type parameters
/// (for a method, its impl's or `Self` first), what `Self` is in it, for a
/// method of an impl, and its impl's lifetime parameters.
struct Decl<'p> {
    f: &'p FnDecl,
    params: Params,
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
    /// The types `==` is used on so far, after `eq_subject`.
    compared: HashSet<Type>,
    /// What projections stand for, by the impls: in a copy, every type is
    /// known, so every one does.
    proj: Projections,
    /// `Program::next_id`: the nodes made here that weren't in the program
    /// take ids from it. A copy of one that was keeps its id, so the copies
    /// of a generic function share theirs.
    next_id: u32,
}

impl<'p> Mono<'p> {
    /// An id for a node made here (see `NodeId`).
    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

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
            compared: HashSet::new(),
            proj: Projections::new(&info.traits),
            next_id: prog.next_id,
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
                self.function(f, &Subst::default(), &Params::default(), None),
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
            mods: prog.mods.clone(),
            core: None,
            alloc: None,
            uses: Vec::new(),
            types: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            fns: Vec::new(),
            stray_docs: Vec::new(),
            next_id: self.next_id,
            proved: prog.proved.clone(),
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
        self.without_unused_core(&mut out);
        out
    }

    /// `out` without the core library's functions, structs and enums that
    /// are made whether they're used or not, as they aren't generic, and
    /// aren't: they'd only be noise in what `t mono` prints. A copy of
    /// anything generic is only made where it's used.
    fn without_unused_core(&self, out: &mut Program) {
        // Copies of the core library's generic types too, which its
        // functions' signatures may make, like `Option<i64>`.
        let lazy = |name: &str| {
            let of = name.split("::<").next().unwrap_or(name);
            // Only the built-in libraries give slices, strings and
            // characters methods, or implement traits for them.
            let builtin = ["<[", "<str", "<char"].iter().any(|p| name.starts_with(p));
            builtin
                || self.info.library_symbols.contains(name)
                || self.info.library_symbols.contains(of)
        };
        let mut used = HashSet::new();
        let mut kept_types: Vec<bool> = out.types.iter().map(|d| !lazy(&d.name().name)).collect();
        let mut kept_fns: Vec<bool> = out.fns.iter().map(|f| !lazy(&f.name.name)).collect();
        let mut seen_types = vec![false; out.types.len()];
        let mut seen_fns = vec![false; out.fns.len()];
        loop {
            let mut changed = false;
            for (i, d) in out.types.iter().enumerate() {
                kept_types[i] |= used.contains(&d.name().name);
                if kept_types[i] && !seen_types[i] {
                    seen_types[i] = true;
                    changed = true;
                    used::in_type_decl(d, &mut used);
                }
            }
            for (i, f) in out.fns.iter().enumerate() {
                kept_fns[i] |= used.contains(&f.name.name);
                if kept_fns[i] && !seen_fns[i] {
                    seen_fns[i] = true;
                    changed = true;
                    used::in_fn(f, &mut used);
                }
            }
            if !changed {
                break;
            }
        }
        let mut keep = kept_types.into_iter();
        out.types.retain(|_| keep.next().unwrap_or(true));
        let mut keep = kept_fns.into_iter();
        out.fns.retain(|_| keep.next().unwrap_or(true));
    }

    /// The name of the copy of function `name` for `args`, `mangled`,
    /// making it if it's new.
    fn named_copy(&mut self, name: &str, args: Vec<Type>, mangled: String) -> String {
        if !self.copies.contains_key(&mangled) {
            self.fn_queue.push(mangled.clone());
            let copy = Copy {
                of: name.to_string(),
                args,
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
    fn function(&mut self, f: &FnDecl, s: &Subst, params: &Params, name: Option<&str>) -> FnDecl {
        let mut params = params.clone();
        params.push_type("Self".into());
        let mut cx = Cx { m: self, s, params };
        FnDecl {
            doc: f.doc.clone(),
            allows: f.allows.clone(),
            span: f.span,
            module: ROOT,
            public: f.public,
            name: Ident {
                name: name.unwrap_or(&f.name.name).to_string(),
                span: f.name.span,
                id: f.name.id,
            },
            generics: lifetimes_only(&f.generics),
            params: f
                .params
                .iter()
                .map(|p| Param {
                    doc: None,
                    allows: Vec::new(),
                    mutable: p.mutable,
                    public: None,
                    name: p.name.clone(),
                    ty: cx.ty(&p.ty),
                })
                .collect(),
            parens: f.parens,
            ret: cx.ty(&f.ret),
            effects: cx.written_effects(&cx.resolved_effects(f.effects.as_ref()), f.span),
            body: cx.expr(&f.body),
            intrinsic: f.intrinsic,
        }
    }

    /// The copy of struct or enum `d` for type arguments `args`.
    fn type_decl(&mut self, d: &TypeDecl, args: &[Type]) -> TypeDecl {
        let params = names(d.generics());
        let s = Subst::new(&params, args);
        let mut cx = Cx {
            m: self,
            s: &s,
            params,
        };
        let name = Ident {
            name: mangle(&d.name().name, args),
            span: d.name().span,
            id: d.name().id,
        };
        // Whether it's `Discard`, `Copy` and `Eq`, which the checker of what
        // this makes takes the copy's word for.
        let ty = Type::Adt(d.name().name.clone(), Vec::new(), args.to_vec());
        let traits = &cx.m.info.traits;
        let names: Vec<Ident> = ["Discard", "Copy", "Eq"]
            .into_iter()
            .filter(|tr| traits.implements(&ty, tr, &[], &[]).is_ok())
            .map(|tr| Ident {
                name: tr.into(),
                span: d.name().span,
                id: cx.m.id(),
            })
            .collect();
        let derives = match names.is_empty() {
            true => Vec::new(),
            false => vec![Derive {
                span: d.name().span,
                wheres: vec![None; names.len()],
                names,
            }],
        };
        match d {
            TypeDecl::Struct(st) => TypeDecl::Struct(StructDecl {
                doc: st.doc.clone(),
                allows: st.allows.clone(),
                span: st.span,
                module: ROOT,
                public: st.public,
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
                        public: f.public,
                        name: f.name.clone(),
                        ty: cx.ty(&f.ty),
                    })
                    .collect(),
                intrinsic: st.intrinsic,
            }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl {
                doc: e.doc.clone(),
                allows: e.allows.clone(),
                span: e.span,
                module: ROOT,
                public: e.public,
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
}

/// The type and effect parameters `g` declares.
fn names(g: &Generics) -> Params {
    Params::new(
        g.types.iter().map(|p| p.name.clone()).collect(),
        g.effect_names(),
    )
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
                params: names(&f.generics),
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
            let params = def.params.concat(&names(&f.generics));
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
            let params = Params::types(vec!["Self".to_string()]).concat(&names(&m.decl.generics));
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
        effects: Vec::new(),
        bounds: g
            .bounds
            .iter()
            .filter(|(p, _)| p.name.starts_with('\''))
            .cloned()
            .collect(),
        span: g.span,
        wheres: Vec::new(),
        where_span: None,
    }
}

/// Copying one function or type.
struct Cx<'m, 'p, 's> {
    m: &'m mut Mono<'p>,
    /// What each type parameter of the one being copied stands for.
    s: &'s Subst,
    /// Its type parameters.
    params: Params,
}

impl Cx<'_, '_, '_> {
    /// A type written in the original, as written in the copy.
    fn ty(&mut self, t: &TypeExpr) -> TypeExpr {
        // A refinement is kept, for the interpreter to check.
        if let TypeExprKind::Refined(v, base, pred) = &t.kind {
            return TypeExpr {
                kind: TypeExprKind::Refined(v.clone(), Box::new(self.ty(base)), pred.clone()),
                span: t.span,
                id: t.id,
            };
        }
        let resolved = self.resolved(t);
        TypeExpr {
            id: t.id,
            ..self.written(&resolved, t.span)
        }
    }

    /// A type written in the original, as it is in the copy.
    fn resolved(&self, t: &TypeExpr) -> Type {
        self.resolving(|names, proj, params| {
            check::resolve(t, names, proj, params, &mut check::Lifetimes::Any)
        })
    }

    /// Effects written in the original, as they are in the copy: `panic`
    /// and `alloc` only.
    fn resolved_effects(&self, e: Option<&EffectSet>) -> Type {
        match e {
            Some(e) => self.resolving(|names, proj, params| {
                check::resolve_effects(e, names, proj, params, &mut check::Lifetimes::Any)
            }),
            None => Type::no_effects(),
        }
    }

    /// What `go` resolves, given what the names in the original mean, as it
    /// is in the copy.
    fn resolving(
        &self,
        go: impl FnOnce(
            &dyn Fn(&str, &TypeExpr) -> Option<check::TypeName>,
            &check::ProjNames,
            &Params,
        ) -> Result<Type, crate::error::Error>,
    ) -> Type {
        // Everything is at the top level by now, named by its symbol.
        let arities = &self.m.arities;
        let opaques = &self.m.info.opaques;
        let names = |name: &str, t: &TypeExpr| {
            if let Some(symbol) = opaques.get(&t.id) {
                let d = &self.m.info.traits.opaques[symbol];
                let (lifetimes, params) = (d.lifetimes.clone(), d.params.clone());
                return Some(check::TypeName::Opaque(symbol.clone(), lifetimes, params));
            }
            match self.params.iter().any(|p| p == name) {
                true => None,
                false => arities
                    .get(name)
                    .map(|&(l, n)| check::TypeName::Adt(name.to_string(), l, n)),
            }
        };
        let projections = &self.m.info.projections;
        let proj = |s: Type, tr: Option<&Ident>, name: &Ident, t: &TypeExpr, _: check::Kind| {
            let tr = match tr {
                Some(tr) => tr.name.clone(),
                None => projections[&t.id].clone(),
            };
            Ok(Type::Proj(Box::new(s), tr, name.name.clone()))
        };
        let resolved =
            go(&names, &proj, &self.params).expect("monomorphising a program that checked");
        self.subst(&resolved)
    }

    /// `t`, of the original, as it is in the copy: with the copy's type
    /// arguments, every opaque type replaced by the type it stands for,
    /// and every projection worked out, as each can be now.
    fn subst(&self, t: &Type) -> Type {
        self.m.proj.reveal(&self.s.apply(t))
    }

    /// Makes the copies of every impl's `eq` that `==` on values of type
    /// `t` may call: the one of `t`'s impl of `Eq`, if it has
    /// one of its own, or else those of the types of its fields, if it's
    /// derived, and so on down. Each type once: a type can hold itself,
    /// through a box. That ends, as the check against polymorphic
    /// recursion sees through derived impls the same way (`dispatch`).
    ///
    /// `t` and the field types have their projections worked out (`subst`,
    /// `resolved`), so it's the impls of what they stand for that it finds.
    fn need_eq(&mut self, t: &Type) {
        let t = eq_subject(t).erased();
        if !matches!(t, Type::Adt(..)) || !self.m.compared.insert(t.clone()) {
            return;
        }
        match self.target("<Eq>::eq", vec![t.clone()]) {
            Target::Fn(of, args, copy) => {
                self.m.named_copy(&of, args, copy);
            }
            Target::Eq(t) => {
                for f in self.field_types(&t) {
                    self.need_eq(&f);
                }
            }
        }
    }

    /// The type of each field of struct or enum type `t`, which has no
    /// type parameters left in it.
    fn field_types(&mut self, t: &Type) -> Vec<Type> {
        let Type::Adt(name, _, args) = t else {
            unreachable!("fields of `{t}`")
        };
        let d = self.m.types[name.as_str()];
        let params = names(d.generics());
        let s = Subst::new(&params, args);
        let cx = Cx {
            m: &mut *self.m,
            s: &s,
            params,
        };
        let written: Vec<&TypeExpr> = match d {
            TypeDecl::Struct(st) => st.fields.iter().map(|f| &f.ty).collect(),
            TypeDecl::Enum(e) => e.variants.iter().flat_map(|v| &v.fields).collect(),
        };
        written.into_iter().map(|f| cx.resolved(f)).collect()
    }

    /// How to write `t`, which has no type parameters left in it.
    fn written(&mut self, t: &Type, span: Span) -> TypeExpr {
        let kind = match t {
            Type::Int(t) => TypeExprKind::Named(t.name().into(), Vec::new()),
            Type::Bool => TypeExprKind::Named("bool".into(), Vec::new()),
            Type::Char => TypeExprKind::Named("char".into(), Vec::new()),
            Type::Str => TypeExprKind::Named("str".into(), Vec::new()),
            Type::Unit => TypeExprKind::Unit,
            Type::Never => TypeExprKind::Never,
            Type::Fn(sig) => TypeExprKind::Fn(
                sig.params.iter().map(|p| self.written(p, span)).collect(),
                Box::new(self.written(&sig.ret, span)),
                self.written_effects(&sig.effects, span),
            ),
            // A closure's type can't be written; this is only for showing
            // the program.
            Type::Adt(name, _, args) if crate::types::is_closure(name) => {
                let args: Vec<Type> = args.iter().map(Type::erased).collect();
                for a in args.iter().filter(|a| !matches!(a, Type::Effects(_))) {
                    self.written(a, span);
                }
                TypeExprKind::Named(mangle(name, &args), Vec::new())
            }
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
                            id: self.m.id(),
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
                        id: self.m.id(),
                    }),
                };
                TypeExprKind::Ref(lifetime, *m, Box::new(self.written(inner, span)))
            }
            Type::Box(inner) => TypeExprKind::Named("Box".into(), vec![self.written(inner, span)]),
            Type::Slice(inner) => TypeExprKind::Slice(Box::new(self.written(inner, span))),
            Type::Param(_) | Type::Var(_) | Type::Proj(..) | Type::Opaque(..) => {
                unreachable!("`{t}` left after monomorphising")
            }
            Type::Effects(_) | Type::Alloc | Type::Panic | Type::Unsafe => {
                unreachable!("effects `{t}` written as a type")
            }
        };
        TypeExpr {
            kind,
            span,
            id: self.m.id(),
        }
    }

    /// How to write effects `t`, which are `alloc`, `panic` and `unsafe` only by now:
    /// nothing, if there are none.
    fn written_effects(&mut self, t: &Type, span: Span) -> Option<EffectSet> {
        let items: Vec<Effect> = t
            .effect_items()
            .iter()
            .map(|e| {
                let name = match e {
                    Type::Alloc | Type::Panic | Type::Unsafe => e.to_string(),
                    e => unreachable!("effect `{e}` left after monomorphising"),
                };
                let id = self.m.id();
                Effect {
                    kind: EffectKind::Named(Ident { name, span, id }),
                    span,
                    id: self.m.id(),
                }
            })
            .collect();
        if items.is_empty() {
            return None;
        }
        Some(EffectSet {
            braced: items.len() > 1,
            items,
            span,
            with: true,
        })
    }

    /// The use of something generic at `span`, if that's what's there, as
    /// the name of its copy.
    fn instance(&mut self, id: NodeId) -> Option<String> {
        let instance = self.m.info.instances.get(&id)?.clone();
        Some(match instance {
            // A copy doesn't depend on lifetimes, so they're left out.
            Instance::Fn(name, args) => {
                let args = args.iter().map(|a| self.subst(a).erased()).collect();
                match self.target(&name, args) {
                    Target::Fn(of, args, copy) => self.m.named_copy(&of, args, copy),
                    Target::Eq(_) => unreachable!("`eq` of a built-in `Eq` is `==`"),
                }
            }
            Instance::Adt(name, args) => {
                let args: Vec<Type> = args.iter().map(|a| self.subst(a).erased()).collect();
                // Its fields' types may need copies of their own.
                self.written(
                    &Type::Adt(name.clone(), Vec::new(), args.clone()),
                    Span::new(0, 0),
                );
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
        match traits.implements(&self_ty, &tr, &[], &[]) {
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
            Type::Proj(..) => unreachable!("`{t}` left after monomorphising"),
            Type::Ref(_, m, inner) => {
                format!("&{}{}", if *m { "mut " } else { "" }, self.copy_name(inner))
            }
            Type::Box(inner) => format!("Box<{}>", self.copy_name(inner)),
            Type::Slice(inner) => format!("[{}]", self.copy_name(inner)),
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
        let node = |kind: ExprKind, m: &mut Mono| Expr {
            kind,
            span,
            id: m.id(),
        };
        let deref = |name: &str, m: &mut Mono| {
            let var = node(ExprKind::Var(name.into(), Vec::new()), m);
            node(ExprKind::Unary(UnOp::Deref, Box::new(var)), m)
        };
        let (a, b) = (deref("a", self.m), deref("b", self.m));
        let body = node(
            ExprKind::Binary(BinOp::Eq, Box::new(a), Box::new(b)),
            self.m,
        );
        let params = ["a", "b"]
            .map(|name| LambdaParam {
                mutable: None,
                name: Ident {
                    name: name.into(),
                    span,
                    id: self.m.id(),
                },
                ty: Some(self.written(&r, span)),
            })
            .into();
        let id = self.m.id();
        ExprKind::Lambda(Arc::new(Lambda::new(id, params, None, body)))
    }

    /// `id`, renamed to its copy if it names something generic.
    fn ident(&mut self, id: &Ident) -> Ident {
        Ident {
            name: self.instance(id.id).unwrap_or_else(|| id.name.clone()),
            span: id.span,
            id: id.id,
        }
    }

    fn expr(&mut self, e: &Expr) -> Expr {
        let kind = match &e.kind {
            ExprKind::For(_) | ExprKind::Range(..) | ExprKind::Try(..) => {
                unreachable!("rewritten when paths are resolved")
            }
            ExprKind::Int(n, _) => {
                let t = self.m.info.literals.get(&e.id).map(|t| self.subst(t));
                match t {
                    Some(Type::Int(t)) => ExprKind::Int(*n, Some(t)),
                    _ => unreachable!("an integer literal of type {t:?}"),
                }
            }
            ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::ByteStr(_)
            | ExprKind::Unit
            | ExprKind::Continue(_) => e.kind.clone(),
            // Type arguments written in the source are in `info.instances`
            // like the inferred ones, and the copy's name says them.
            ExprKind::Var(name, _) => match self.m.info.instances.get(&e.id).cloned() {
                Some(Instance::Fn(f, args)) => {
                    let args = args.iter().map(|a| self.subst(a).erased()).collect();
                    match self.target(&f, args) {
                        Target::Fn(of, args, copy) => {
                            let name = self.m.named_copy(&of, args, copy);
                            ExprKind::Var(name, Vec::new())
                        }
                        Target::Eq(t) => {
                            self.need_eq(&t);
                            self.eq_fn(&t, e.span)
                        }
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
                            let name = self.m.named_copy(&of, args, copy);
                            ExprKind::Var(name, Vec::new())
                        }
                        Target::Eq(_) => unreachable!("an inherent method"),
                    }
                }
                _ => ExprKind::Var(
                    self.instance(e.id).unwrap_or_else(|| name.clone()),
                    Vec::new(),
                ),
            },
            ExprKind::Unary(op, x) => ExprKind::Unary(*op, Box::new(self.expr(x))),
            ExprKind::As(x, t) => ExprKind::As(Box::new(self.expr(x)), Box::new(self.ty(t))),
            ExprKind::Binary(op, l, r) => {
                if let Some(Instance::Fn(_, args)) = self.m.info.instances.get(&e.id).cloned() {
                    self.need_eq(&self.subst(&args[0]));
                }
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
                let mut lambda = Lambda::new(l.id, params, ret, self.expr(&l.body));
                lambda.moves = l.moves;
                lambda.modes = l.modes.clone();
                ExprKind::Lambda(Arc::new(lambda))
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
            ExprKind::Unsafe(block) => ExprKind::Unsafe(Box::new(self.expr(block))),
            ExprKind::Move(x) => ExprKind::Move(Box::new(self.expr(x))),
            ExprKind::Return(x) => ExprKind::Return(x.as_ref().map(|x| Box::new(self.expr(x)))),
            ExprKind::Break(label, x) => {
                ExprKind::Break(label.clone(), x.as_ref().map(|x| Box::new(self.expr(x))))
            }
        };
        Expr {
            kind,
            span: e.span,
            id: e.id,
        }
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
            PatternKind::Move(p) => PatternKind::Move(Box::new(self.pattern(p))),
            kind => kind.clone(),
        };
        Pattern {
            kind,
            span: p.span,
            id: p.id,
        }
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

/// The names a monomorphised item uses: of functions, structs and enums.
mod used {
    use crate::ast::*;
    use std::collections::HashSet;

    pub(super) fn in_type_decl(d: &TypeDecl, out: &mut HashSet<String>) {
        match d {
            TypeDecl::Struct(s) => s.fields.iter().for_each(|f| in_type(&f.ty, out)),
            TypeDecl::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| &v.fields)
                .for_each(|t| in_type(t, out)),
        }
    }

    pub(super) fn in_fn(f: &FnDecl, out: &mut HashSet<String>) {
        f.params.iter().for_each(|p| in_type(&p.ty, out));
        in_type(&f.ret, out);
        in_expr(&f.body, out);
    }

    fn in_type(t: &TypeExpr, out: &mut HashSet<String>) {
        match &t.kind {
            TypeExprKind::Named(name, args) => {
                out.insert(name.clone());
                args.iter().for_each(|a| in_type(a, out));
            }
            TypeExprKind::Fn(params, ret, effects) => {
                params.iter().for_each(|p| in_type(p, out));
                in_type(ret, out);
                effects
                    .iter()
                    .flat_map(EffectSet::types)
                    .for_each(|t| in_type(t, out));
            }
            TypeExprKind::Ref(_, _, inner) | TypeExprKind::Slice(inner) => in_type(inner, out),
            TypeExprKind::Proj(inner, ..) | TypeExprKind::Refined(_, inner, _) => {
                in_type(inner, out)
            }
            TypeExprKind::Impl(_)
            | TypeExprKind::Unit
            | TypeExprKind::Never
            | TypeExprKind::Lifetime(_) => {}
        }
    }

    fn in_pattern(p: &Pattern, out: &mut HashSet<String>) {
        match &p.kind {
            PatternKind::Variant(ty, _, _, pats) => {
                out.insert(ty.name.clone());
                pats.iter().for_each(|p| in_pattern(p, out));
            }
            PatternKind::Struct(ty, _, fields, _) => {
                out.insert(ty.name.clone());
                fields.iter().for_each(|f| in_pattern(&f.pat, out));
            }
            PatternKind::Or(alts) => alts.iter().for_each(|p| in_pattern(p, out)),
            _ => {}
        }
    }

    fn in_expr(e: &Expr, out: &mut HashSet<String>) {
        match &e.kind {
            ExprKind::Var(name, _) => {
                out.insert(name.clone());
            }
            ExprKind::StructLit(ty, ..) | ExprKind::Variant(ty, ..) => {
                out.insert(ty.name.clone());
            }
            ExprKind::Match(_, arms) => arms.iter().for_each(|a| in_pattern(&a.pat, out)),
            ExprKind::Block(stmts, _) => {
                for s in stmts {
                    if let Stmt::Let { pat, ty, .. } = s {
                        in_pattern(pat, out);
                        ty.iter().for_each(|t| in_type(t, out));
                    }
                }
            }
            ExprKind::Lambda(l) => {
                l.params
                    .iter()
                    .flat_map(|p| &p.ty)
                    .for_each(|t| in_type(t, out));
                l.ret.iter().for_each(|t| in_type(t, out));
            }
            _ => {}
        }
        e.children().into_iter().for_each(|c| in_expr(c, out));
    }
}
