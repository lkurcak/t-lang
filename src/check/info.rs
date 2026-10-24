//! What the checker learns about each name in a program, kept for later:
//! the type of every variable and use, what each name refers to and its doc
//! comment, and which type arguments each use of something generic has.
//! Editors show it (hover, inlay hints, go to definition, references and
//! rename, see `lsp.rs` and `rename.rs`), and monomorphisation reads the
//! type arguments (see `mono.rs`).

use super::modules::Item;
use super::{Checker, Lifetimes};
use crate::ast::*;
use crate::error::Span;
use crate::types::{Type, comma_list};
use std::collections::{HashMap, HashSet};

/// Everything the checker found out, as far as it got.
#[derive(Debug, Clone, Default)]
pub struct Info {
    /// The operators proved not to panic, by node (see `check/refine.rs`),
    /// which the interpreter checks.
    pub proved: std::collections::HashSet<NodeId>,
    /// The type of each integer literal, by node, which in a generic
    /// function can be a type parameter, for monomorphisation to write.
    pub literals: HashMap<NodeId, Type>,
    /// One per name in the program that means something: variables where
    /// they're bound and where they're used, functions, types, fields and
    /// variants. In no particular order.
    pub notes: Vec<Note>,
    /// The type arguments of each use of a generic function, struct or enum,
    /// by its node (see `ast::NodeId`): the expression that names a
    /// function, or the name of a method or of a struct or enum. In a
    /// generic function these can mention its own type parameters.
    pub instances: HashMap<NodeId, Instance>,
    /// The comparisons `<`, `<=`, `>` and `>=` that are calls of `Ord`'s
    /// methods, by node: those not of an integer type, whose are the
    /// machine's. Each has its method's instance in `instances`.
    pub ord_calls: std::collections::HashSet<NodeId>,
    /// The symbol of every function, struct and enum, by the node of its
    /// name where it's declared and where it's used: a name for it that's
    /// unique in the program, unlike the one it's written with (see
    /// `check/scope.rs` and `check/names.rs`).
    pub symbols: HashMap<NodeId, String>,
    /// Where each struct and enum is declared, by symbol: the span of its
    /// name.
    pub types: HashMap<String, Span>,
    /// The references the checker made into others, by the expression:
    /// `r` used as a `&T` when it's a `&Box<T>`, a `&&T` or a
    /// `&mut T`, with how many more `*`s it takes to get to the `T`, and
    /// whether the result is `&mut`. `lift` writes them out: `&**r`.
    pub coercions: HashMap<NodeId, (usize, bool)>,
    /// Every trait and impl, for monomorphisation to find impls by.
    pub traits: crate::traits::Traits,
    /// The method each method call and path names, by the node of its
    /// name, for `lift` to write out as a plain call.
    pub methods: HashMap<NodeId, Resolved>,
    /// The trait of each projection written, by its node: `Iter` for a
    /// `T::Item` where `T: Iter`, for monomorphisation to work it out by.
    pub projections: HashMap<NodeId, String>,
    /// How each closure that captures captures each variable it does, by
    /// the closure, for `lift` to write into it.
    pub captures: HashMap<NodeId, Vec<(String, crate::ast::Capture)>>,
    /// How each call of what isn't a function type gets at what it calls,
    /// by the call, for `lift` to write out: `f(1)` for a
    /// closure `f` that's `FnMut` is `(&mut f)(1)`. Through how many `*`s,
    /// and by which `Fn` trait.
    pub calls: HashMap<NodeId, (usize, crate::traits::FnKind)>,
    /// The symbol of each opaque type, `impl Tr` in a return type, by the
    /// node it's written as, for monomorphisation to replace it with what
    /// it stands for.
    pub opaques: HashMap<NodeId, String>,
    /// The symbols of the built-in libraries' functions, structs and enums,
    /// which monomorphisation keeps only if the program uses them.
    pub library_symbols: HashSet<String>,
    /// Where the borrow checker moved a value rather than copying it, and
    /// each pattern that moves out of what it matches, for `lift` to write
    /// out (`ExprKind::Move`, `PatternKind::Move`). It decides by the types
    /// it knows, so in generic code by the bounds: a value of a type
    /// parameter that isn't known to be `Copy` moves, whatever type it
    /// turns out to be. Then the program runs as it was checked, and means
    /// the same at every type, as it would if it ran once for them all.
    pub moves: HashSet<MoveSite>,
    /// The error type of the `Result` `main` gives back, if it does: the
    /// type monomorphisation makes `std::rt::report` for, which reports
    /// it.
    pub main_error: Option<Type>,
}

/// Where a value is used by value, which moves it or copies it, as
/// `Info::moves` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveSite {
    /// A place used as a value.
    Expr(NodeId),
    /// The receiver of a call, by the call: what's before the `.` of a
    /// method call whose method takes `self`, or a closure called as an
    /// `FnOnce`.
    Receiver(NodeId),
    /// The `i`th variable a closure captures, captured by value, by the
    /// closure.
    Capture(NodeId, usize),
    /// A binding by value, which moves into its variable, or a struct or
    /// enum pattern, which uses up what it takes apart.
    Pattern(NodeId),
}

/// What a method call `x.m(..)` or path `T::m` names: method `symbol`.
/// For a call, how many `*`s it took to get from `x` to what the method is
/// of, and whether a `&` (`Some(false)`) or `&mut` (`Some(true)`) was
/// then taken of that, as the method's receiver wants: `x.len()` is
/// `<Point>::len(&*x)` for `x: &Point`.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub symbol: String,
    pub derefs: usize,
    pub autoref: Option<bool>,
}

impl Info {
    /// The name at `offset`, if there's one: the innermost. In a shorthand
    /// `P { x }`, which names both a field and a variable, the variable.
    pub fn note_at(&self, offset: usize) -> Option<&Note> {
        self.notes
            .iter()
            .filter(|n| n.span.contains(offset))
            .min_by_key(|n| {
                (
                    n.span.end - n.span.start,
                    !matches!(n.kind, NoteKind::Var(_)),
                )
            })
    }

    /// Every place that names what's declared at `def`: the declaration and
    /// every use, in order. What counts is what each name refers to, not
    /// how it's spelled, so a nested `fn` or local type is told apart from
    /// another of the same name, and a variable from one it shadows or that
    /// shadows it.
    pub fn occurrences(&self, def: Span) -> Vec<Span> {
        let mut out: Vec<Span> = self
            .notes
            .iter()
            .filter(|n| n.def == Some(def))
            .map(|n| n.span)
            .chain([def])
            .collect();
        out.sort_by_key(|s| (s.start, s.end));
        out.dedup();
        out
    }
}

/// A use of something generic, with its type arguments: `id` in `id(1)` is
/// `Fn("id", [i64])`. Each is named by its symbol.
#[derive(Debug, Clone, PartialEq)]
pub enum Instance {
    Fn(String, Vec<Type>),
    Adt(String, Vec<Type>),
}

/// What the name at `span` is.
#[derive(Debug, Clone)]
pub struct Note {
    pub span: Span,
    pub kind: NoteKind,
    /// Its type: a variable's or a field's, a function's (instantiated, if
    /// it's generic), a variant's as a constructor, or a struct or enum
    /// itself.
    pub ty: Option<Type>,
    /// Its type arguments, if it's a use of something generic.
    pub args: Vec<Type>,
    /// Where it's defined, if anywhere: a built-in isn't.
    pub def: Option<Span>,
    /// The doc comment where it's defined, as Markdown, if it has one.
    pub doc: Option<String>,
    /// Whether nothing in the source says what `ty` is, or for a use of
    /// something generic, what `args` are, so an editor might show it: a
    /// `let` without an annotation, say, or `id(1)` without `::<i64>`.
    pub inferred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteKind {
    Var(String),
    Fn(String),
    Trait(String),
    Struct(String),
    Enum(String),
    TypeParam(String),
    EffectParam(String),
    /// By its path: `alloc::list`.
    Module(String),
    Field(String),
    /// Enum, variant.
    Variant(String, String),
}

/// A note for a name written as a path, `a::b::f` at `span`, is for its
/// last segment, `f`, which is what renaming it changes: the path ends
/// where `span` does. A trait is named by its symbol, `a::Show`, however
/// it's written, so the same goes for it.
fn last_segment(span: Span, kind: NoteKind) -> (Span, NoteKind) {
    let (NoteKind::Fn(name)
    | NoteKind::Trait(name)
    | NoteKind::Struct(name)
    | NoteKind::Enum(name)) = &kind
    else {
        return (span, kind);
    };
    let Some((_, last)) = name.rsplit_once("::") else {
        return (span, kind);
    };
    let last = last.to_string();
    let span = Span::new(
        span.end.saturating_sub(last.len()).max(span.start),
        span.end,
    );
    let kind = match kind {
        NoteKind::Fn(_) => NoteKind::Fn(last),
        NoteKind::Trait(_) => NoteKind::Trait(last),
        NoteKind::Struct(_) => NoteKind::Struct(last),
        _ => NoteKind::Enum(last),
    };
    (span, kind)
}

impl Note {
    /// A short description, for hovering over it in an editor.
    pub fn hover(&self) -> String {
        let ty = self.ty.as_ref().map_or("_".to_string(), Type::to_string);
        let args = match self.args.as_slice() {
            [] => String::new(),
            args => format!("::<{}>", comma_list(args)),
        };
        match &self.kind {
            NoteKind::Var(name) => format!("{name}: {ty}"),
            NoteKind::Fn(name) => format!("fn {name}{args}: {ty}"),
            NoteKind::Trait(name) => format!("trait {name}"),
            NoteKind::Struct(_) => format!("struct {ty}"),
            NoteKind::Enum(_) => format!("enum {ty}"),
            NoteKind::TypeParam(name) => format!("type parameter {name}"),
            NoteKind::EffectParam(name) => format!("effect parameter {name}"),
            NoteKind::Module(path) => format!("mod {path}"),
            NoteKind::Field(name) => format!("field {name}: {ty}"),
            NoteKind::Variant(e, v) => format!("{e}{args}::{v}: {ty}"),
        }
    }

    /// What an editor might show right after it, if anything: the type of a
    /// variable whose type was inferred (`: i64`), or the type arguments of a
    /// use of something generic (`::<i64>`). Without lifetimes: inferred
    /// ones are the borrow checker's business, and have no names.
    pub fn hint(&self) -> Option<String> {
        if !self.inferred {
            return None;
        }
        match self.kind {
            NoteKind::Fn(_) | NoteKind::Struct(_) | NoteKind::Enum(_) => {
                let args: Vec<Type> = self.args.iter().map(Type::erased).collect();
                Some(format!("::<{}>", comma_list(&args)))
            }
            _ => self.ty.as_ref().map(|t| format!(": {}", t.erased())),
        }
    }
}

/// Where each type, field and variant is declared, by the type's symbol.
/// The checker keeps track of functions itself.
#[derive(Clone, Debug, Default)]
pub(super) struct Defs {
    pub(super) types: HashMap<String, Span>,
    /// By `(type, field)`.
    pub(super) fields: HashMap<(String, String), Span>,
    /// By `(enum, variant)`.
    pub(super) variants: HashMap<(String, String), Span>,
    /// The doc comment of each function, type, field and variant that has
    /// one, as Markdown, by where it's declared.
    pub(super) docs: HashMap<Span, String>,
}

impl Defs {
    /// Adds those of the package whose items come after `from` (see
    /// `Counts`): the first declaration of each, as the checker rejects the
    /// others. `types` are its types, in the order of `Program::all_types`.
    pub(super) fn add(&mut self, prog: &Program, types: &[super::TypeDef], from: Counts) {
        let defs = self;
        for (i, m) in prog.mods.iter().enumerate().skip(from.mods) {
            if let Some(d) = &m.doc {
                defs.docs.insert(module_def(prog, i), d.markdown());
            }
        }
        let mut doc = |name: &Ident, doc: Option<&DocComment>| {
            if let Some(d) = doc {
                defs.docs.insert(name.span, d.markdown());
            }
        };
        for (f, ..) in prog.all_fns().into_iter().skip(from.all_fns) {
            doc(&f.name, f.doc.as_ref());
        }
        for t in &prog.traits[from.traits..] {
            doc(&t.name, t.doc.as_ref());
        }
        let all: Vec<_> = prog.all_types().into_iter().skip(from.all_types).collect();
        for (d, _) in &all {
            doc(d.name(), d.doc());
            match d {
                TypeDecl::Struct(s) => s.fields.iter().for_each(|f| doc(&f.name, f.doc.as_ref())),
                TypeDecl::Enum(e) => e.variants.iter().for_each(|v| doc(&v.name, v.doc.as_ref())),
            }
        }
        for ((d, _), def) in all.iter().zip(types) {
            let ty = def.symbol.clone();
            defs.types.entry(ty.clone()).or_insert(d.name().span);
            match d {
                TypeDecl::Struct(s) => {
                    for f in &s.fields {
                        let key = (ty.clone(), f.name.name.clone());
                        defs.fields.entry(key).or_insert(f.name.span);
                    }
                }
                TypeDecl::Enum(e) => {
                    for v in &e.variants {
                        let key = (ty.clone(), v.name.name.clone());
                        defs.variants.entry(key).or_insert(v.name.span);
                    }
                }
            }
        }
    }
}

impl Checker {
    /// Notes what's at `span`. Its types may still have variables in them:
    /// `finish_notes` solves them once the function is checked.
    pub(super) fn note(&mut self, span: Span, kind: NoteKind, ty: Option<Type>, def: Option<Span>) {
        let (span, kind) = last_segment(span, kind);
        if unwritten(span, &kind) {
            return;
        }
        self.info.notes.push(Note {
            span,
            kind,
            ty,
            args: Vec::new(),
            def,
            doc: self.doc(def),
            inferred: false,
        });
    }

    /// The doc comment of what's defined at `def`.
    fn doc(&self, def: Option<Span>) -> Option<String> {
        self.defs.docs.get(&def?).cloned()
    }

    /// Like `note`, for a use of something with type arguments `args`, if
    /// it's generic, which monomorphisation will need, by the node of the
    /// use, with the span of the whole of it: `id::<i64>` rather than just
    /// `id`. Uses of things that
    /// aren't generic need nothing. The type arguments were `inferred`
    /// unless they were written out.
    pub(super) fn note_use(
        &mut self,
        span: Span,
        kind: NoteKind,
        ty: Type,
        def: Option<Span>,
        instance: Option<(NodeId, Span, Instance)>,
        inferred: bool,
    ) {
        let args = match &instance {
            Some((_, _, Instance::Fn(_, args) | Instance::Adt(_, args))) => args.clone(),
            None => Vec::new(),
        };
        if let Some(instance) = instance
            && !args.is_empty()
        {
            self.pending.push(instance);
        }
        let (span, kind) = last_segment(span, kind);
        if unwritten(span, &kind) {
            return;
        }
        self.info.notes.push(Note {
            span,
            kind,
            ty: Some(ty),
            inferred: inferred && !args.is_empty(),
            args,
            def,
            doc: self.doc(def),
        });
    }

    /// Notes the variables in `binds`, which a pattern just bound. Their
    /// types were `inferred` unless the pattern had an annotation.
    pub(super) fn note_binds(&mut self, binds: &[super::Local], inferred: bool) {
        for l in binds {
            for site in &l.sites {
                if unwritten(site.span, &NoteKind::Var(l.name.clone())) {
                    continue;
                }
                self.info.notes.push(Note {
                    span: site.span,
                    kind: NoteKind::Var(l.name.clone()),
                    ty: Some(l.ty.clone()),
                    args: Vec::new(),
                    def: Some(l.sites[0].span),
                    doc: None,
                    // `P { x }` binds `x`, but `P { x: i64 }` would read as
                    // a field holding the pattern `i64`.
                    inferred: inferred && !site.shorthand,
                });
            }
        }
    }

    /// At the end of a function, checked or not: solves the variables in
    /// the notes made since `notes` and in the pending instances, as far as
    /// they're known, with the lifetimes of closures' results worked out,
    /// and files the instances in `info`.
    pub(super) fn finish_notes(&mut self, notes: usize) {
        for i in notes..self.info.notes.len() {
            let n = &self.info.notes[i];
            let ty = n.ty.as_ref().map(|t| self.decided_regions(&self.solved(t)));
            let args = n
                .args
                .iter()
                .map(|t| self.decided_regions(&self.solved(t)))
                .collect();
            let n = &mut self.info.notes[i];
            n.ty = ty;
            n.args = args;
        }
        for (id, span, instance) in std::mem::take(&mut self.pending) {
            // A comparison of integers is the machine's, not a call.
            if let Some(t) = self.compares.remove(&id) {
                if self.int_like(&self.solved(&t)) {
                    continue;
                }
                self.info.ord_calls.insert(id);
            }
            let instance = match instance {
                Instance::Fn(name, args) => Instance::Fn(
                    name,
                    args.iter()
                        .map(|t| self.decided_regions(&self.solved(t)))
                        .collect(),
                ),
                Instance::Adt(name, args) => Instance::Adt(
                    name,
                    args.iter()
                        .map(|t| self.decided_regions(&self.solved(t)))
                        .collect(),
                ),
            };
            self.info.instances.insert(id, instance);
            self.instance_at.insert(id, span);
        }
    }

    /// Notes every type written in the program, and every declaration of a
    /// type, type parameter, field or variant. None of that needs inference,
    /// so it's done for the whole program, even past an error.
    pub(super) fn type_notes(&mut self, prog: &Program) {
        let from = self.from;
        let types = prog.all_types().into_iter().enumerate();
        for (i, (d, _)) in types.skip(from.all_types) {
            let params = d.type_params();
            let name = &d.name().name;
            let symbol = &self.types[i].symbol;
            let generic = match self.adts.get(symbol) {
                Some(def) => def.generic(symbol),
                None => Type::Adt(
                    symbol.clone(),
                    Vec::new(),
                    params.iter().map(|p| Type::Param(p.name.clone())).collect(),
                ),
            };
            let kind = match d {
                TypeDecl::Struct(_) => NoteKind::Struct(name.clone()),
                TypeDecl::Enum(_) => NoteKind::Enum(name.clone()),
            };
            let span = d.name().span;
            self.note(span, kind, Some(generic.clone()), Some(span));
            self.param_notes(d.generics());
            self.bound_notes(d.generics(), params);
            // `where T: Discard` in a `#[derive(..)]`.
            for w in d.derives().iter().flat_map(|d| &d.wheres).flatten() {
                for (subject, items) in &w.bounds {
                    if let Some(p) = params.iter().find(|p| p.name == subject.name) {
                        let kind = NoteKind::TypeParam(p.name.clone());
                        self.note(subject.span, kind, None, Some(p.span));
                    }
                    for t in items.iter().flat_map(|b| b.types()) {
                        self.type_expr_notes(t, params);
                    }
                }
            }
            match d {
                TypeDecl::Struct(s) => {
                    for f in &s.fields {
                        let ty = self.resolve_with(&f.ty, &names(params), &mut Lifetimes::Any);
                        let kind = NoteKind::Field(f.name.name.clone());
                        self.note(f.name.span, kind, ty.ok(), Some(f.name.span));
                        self.type_expr_notes(&f.ty, params);
                    }
                }
                TypeDecl::Enum(e) => {
                    for v in &e.variants {
                        let fields: Result<Vec<Type>, _> = v
                            .fields
                            .iter()
                            .map(|t| self.resolve_with(t, &names(params), &mut Lifetimes::Any))
                            .collect();
                        let ty = fields.ok().map(|fields| match fields.is_empty() {
                            true => generic.clone(),
                            false => Type::func(fields, generic.clone()),
                        });
                        let kind = NoteKind::Variant(name.clone(), v.name.name.clone());
                        self.note(v.name.span, kind, ty, Some(v.name.span));
                        for t in &v.fields {
                            self.type_expr_notes(t, params);
                        }
                    }
                }
            }
        }
        // What each `use` names.
        let leaves = self.modules.leaf_items[self.leaves_from..].to_vec();
        for (name, items) in leaves {
            for item in items {
                let (kind, ty, def) = match item {
                    Item::Fn(i) => {
                        // Not known if checking stopped before signatures.
                        let Some(f) = self.fns.get(i) else {
                            continue;
                        };
                        (
                            NoteKind::Fn(f.name.name.clone()),
                            Some(f.scheme.ty.clone()),
                            f.name.span,
                        )
                    }
                    Item::Type(k) => {
                        let d = &prog.types[k];
                        let symbol = &self.types[k].symbol;
                        let ty = self.adts.get(symbol).map(|def| def.generic(symbol));
                        let kind = match d {
                            TypeDecl::Struct(_) => NoteKind::Struct(d.name().name.clone()),
                            TypeDecl::Enum(_) => NoteKind::Enum(d.name().name.clone()),
                        };
                        (kind, ty, d.name().span)
                    }
                    Item::Trait(k) => {
                        let t = &prog.traits[k];
                        (NoteKind::Trait(t.name.name.clone()), None, t.name.span)
                    }
                    Item::Mod(m) => {
                        let kind = NoteKind::Module(self.modules.full_path(m));
                        (kind, None, module_def(prog, m))
                    }
                };
                self.note(name.span, kind, ty, Some(def));
            }
        }
        // Modules where they're declared, and at the start of paths.
        for (i, m) in prog.mods.iter().enumerate().skip(from.mods) {
            if m.parent.is_some() && !prog.is_library_root(i) {
                let kind = NoteKind::Module(self.modules.full_path(i));
                self.note(m.name.span, kind, None, Some(m.name.span));
            }
        }
        let mut refs = self.modules.module_refs[self.refs_from..].to_vec();
        refs.sort_by_key(|(s, _)| s.start);
        refs.dedup();
        for (span, m) in refs {
            let kind = NoteKind::Module(self.modules.full_path(m));
            self.note(span, kind, None, Some(module_def(prog, m)));
        }
        // A method can use its impl's type parameters too.
        let mut outer: HashMap<NodeId, Vec<Ident>> = HashMap::new();
        for d in &prog.impls[from.impls..] {
            let params = &d.generics.types;
            self.param_notes(&d.generics);
            self.type_expr_notes(&d.self_ty, params);
            self.bound_notes(&d.generics, params);
            for a in &d.assoc {
                self.type_expr_notes(&a.ty, params);
            }
            for e in &d.effects {
                self.effect_notes(Some(&e.effects), params);
            }
            for f in &d.methods {
                outer.insert(f.name.id, params.clone());
            }
        }
        for (f, ..) in prog.all_fns().into_iter().skip(from.all_fns) {
            let mut params = outer.get(&f.name.id).cloned().unwrap_or_default();
            params.extend(f.generics.types.iter().cloned());
            let params = &params;
            self.param_notes(&f.generics);
            self.bound_notes(&f.generics, params);
            for p in &f.params {
                self.type_expr_notes(&p.ty, params);
            }
            self.type_expr_notes(&f.ret, params);
            self.effect_notes(f.effects.as_ref(), params);
            each_annotation(&f.body, &mut |t| self.type_expr_notes(t, params));
        }
        for t in &prog.traits[from.traits..] {
            let kind = NoteKind::Trait(t.name.name.clone());
            self.note(t.name.span, kind, None, Some(t.name.span));
            for m in t.methods.iter().filter(|m| !m.provided) {
                let f = &m.decl;
                let params = &f.generics.types;
                self.param_notes(&f.generics);
                let ty = self
                    .traits
                    .method(&t.name.name, &f.name.name)
                    .map(|m| m.scheme.ty.clone());
                let kind = NoteKind::Fn(f.name.name.clone());
                self.note(f.name.span, kind, ty, Some(f.name.span));
                self.bound_notes(&f.generics, params);
                for p in &f.params {
                    self.type_expr_notes(&p.ty, params);
                }
                self.type_expr_notes(&f.ret, params);
                self.effect_notes(f.effects.as_ref(), params);
            }
        }
    }

    /// Notes the names in effects `e`, if there are any, written where type
    /// and effect parameters `params` are in scope: the effect parameters,
    /// and the types of the associated effects.
    fn effect_notes(&mut self, e: Option<&EffectSet>, params: &[Ident]) {
        for t in e.iter().flat_map(|e| e.types()) {
            self.type_expr_notes(t, params);
        }
        self.effect_param_notes(e, params);
    }

    /// Notes the effect parameters in effects `e`, if there are any.
    fn effect_param_notes(&mut self, e: Option<&EffectSet>, params: &[Ident]) {
        for item in e.iter().flat_map(|e| &e.items) {
            if let EffectKind::Named(n) = &item.kind
                && let Some(p) = params.iter().find(|p| p.name == n.name)
            {
                let kind = NoteKind::EffectParam(n.name.clone());
                self.note(n.span, kind, None, Some(p.span));
            }
        }
    }

    /// Notes the types in the bounds of `g`, and in its `where` clause.
    fn bound_notes(&mut self, g: &Generics, params: &[Ident]) {
        for (_, items) in &g.bounds {
            for b in items {
                self.bound_item_notes(b, params);
            }
        }
        for (subject, items) in &g.wheres {
            self.type_expr_notes(subject, params);
            for b in items {
                self.bound_item_notes(b, params);
            }
        }
    }

    /// Notes the types in bound `b`, and its effects.
    fn bound_item_notes(&mut self, b: &BoundItem, params: &[Ident]) {
        // Those of its effects' associated effects among them.
        for t in b.types() {
            self.type_expr_notes(t, params);
        }
        self.effect_param_notes(b.effects.as_ref(), params);
    }

    fn param_notes(&mut self, g: &Generics) {
        for p in &g.types {
            let kind = match g.is_effect(p) {
                true => NoteKind::EffectParam(p.name.clone()),
                false => NoteKind::TypeParam(p.name.clone()),
            };
            self.note(p.span, kind, None, Some(p.span));
        }
    }

    /// Notes the names in type `t`, written where type parameters `params`
    /// are in scope.
    fn type_expr_notes(&mut self, t: &TypeExpr, params: &[Ident]) {
        match &t.kind {
            TypeExprKind::Refined(_, base, _) => self.type_expr_notes(base, params),
            TypeExprKind::Proj(inner, tr, _) => {
                self.type_expr_notes(inner, params);
                if let Some(tr) = tr
                    && let Some(def) = self.traits.traits.get(&tr.name)
                {
                    let (kind, def) = (NoteKind::Trait(tr.name.clone()), def.span);
                    self.note(tr.span, kind, None, def);
                }
            }
            TypeExprKind::Named(name, args) => {
                // Only the name, not the arguments after it.
                let span = Span::new(t.span.start, t.span.start + name.len());
                if let Some(symbol) = self.adt_symbol(t.id) {
                    let def = self.defs.types.get(symbol).copied();
                    let kind = match self.adts.get(symbol).map(|d| &d.kind) {
                        Some(crate::types::AdtKind::Enum(_)) => NoteKind::Enum(name.clone()),
                        _ => NoteKind::Struct(name.clone()),
                    };
                    let ty = self.resolve_with(t, &names(params), &mut Lifetimes::Any);
                    self.note(span, kind, ty.ok(), def);
                } else if let Some(p) = params.iter().find(|p| p.name == *name) {
                    let kind = NoteKind::TypeParam(name.clone());
                    self.note(span, kind, None, Some(p.span));
                }
                for a in args {
                    self.type_expr_notes(a, params);
                }
            }
            TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => {}
            TypeExprKind::Fn(ps, r, effects) => {
                for p in ps {
                    self.type_expr_notes(p, params);
                }
                self.type_expr_notes(r, params);
                self.effect_notes(effects.as_ref(), params);
            }
            TypeExprKind::Ref(_, _, inner) | TypeExprKind::Slice(inner) => {
                self.type_expr_notes(inner, params)
            }
            // Its traits are noted where they're resolved.
            TypeExprKind::Impl(bounds) => {
                for b in bounds {
                    self.bound_item_notes(b, params);
                }
            }
        }
    }
}

/// Is a name at `span` one the program doesn't have, but resolving paths
/// made in rewriting a `for` loop or range (see `modules.rs`)? Those get
/// empty spans, or names no program can write, and nothing to show.
fn unwritten(span: Span, kind: &NoteKind) -> bool {
    span.start == span.end || matches!(kind, NoteKind::Var(n) if n.starts_with('{'))
}

/// Where module `m` is declared: its name, or the start of its file for a
/// package's root, which has none.
pub(super) fn module_def(prog: &Program, m: usize) -> Span {
    let d = &prog.mods[m];
    match d.parent.is_none() || prog.is_library_root(m) {
        false => d.name.span,
        true => Span::new(d.span.start, d.span.start),
    }
}

/// For notes only, so all as type parameters: what an effect parameter
/// can't be resolves to nothing either way.
fn names(params: &[Ident]) -> crate::types::Params {
    crate::types::Params::types(params.iter().map(|p| p.name.clone()).collect())
}

/// Calls `f` on every type written inside `e`: annotations on `let`s and
/// closures, and type arguments, in expressions and in patterns. Not in
/// the functions nested in it.
fn each_annotation(e: &Expr, f: &mut impl FnMut(&TypeExpr)) {
    match &e.kind {
        ExprKind::Move(_) => unreachable!("written by lift"),
        ExprKind::For(_) | ExprKind::Range(..) | ExprKind::Try(..) => {
            unreachable!("rewritten when paths are resolved")
        }
        ExprKind::Int(..)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::ByteStr(_)
        | ExprKind::Unit
        | ExprKind::Continue(_) => {}
        ExprKind::Var(_, targs) => targs.iter().for_each(&mut *f),
        ExprKind::As(x, t) => {
            each_annotation(x, f);
            f(t);
        }
        ExprKind::Unary(_, x)
        | ExprKind::Field(x, _)
        | ExprKind::Loop(_, x)
        | ExprKind::Unsafe(x) => each_annotation(x, f),
        ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter().for_each(|x| each_annotation(x, f)),
        ExprKind::Binary(_, l, r) | ExprKind::Assign(_, l, r) | ExprKind::While(_, l, r) => {
            each_annotation(l, f);
            each_annotation(r, f);
        }
        ExprKind::Call(callee, args) => {
            each_annotation(callee, f);
            args.iter().for_each(|a| each_annotation(a, f));
        }
        ExprKind::MethodCall(x, _, targs, args) => {
            each_annotation(x, f);
            targs.iter().for_each(&mut *f);
            args.iter().for_each(|a| each_annotation(a, f));
        }
        ExprKind::If(c, then, els) => {
            each_annotation(c, f);
            each_annotation(then, f);
            els.iter().for_each(|e| each_annotation(e, f));
        }
        ExprKind::Block(stmts, tail) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Let { pat, ty, init, .. } => {
                        pattern_annotations(pat, f);
                        if let Some(t) = ty {
                            f(t);
                        }
                        init.iter().for_each(|e| each_annotation(e, f));
                    }
                    Stmt::Expr(e) | Stmt::BlockLike(e) => each_annotation(e, f),
                    // A function or type of its own, with its own type
                    // parameters.
                    Stmt::Fn(_) | Stmt::Type(_) => {}
                }
            }
            tail.iter().for_each(|t| each_annotation(t, f));
        }
        ExprKind::StructLit(_, targs, inits) => {
            targs.iter().for_each(&mut *f);
            inits.iter().for_each(|i| each_annotation(&i.value, f));
        }
        ExprKind::Variant(_, targs, _, args, ..) => {
            targs.iter().for_each(&mut *f);
            args.iter().for_each(|a| each_annotation(a, f));
        }
        ExprKind::Match(scrutinee, arms) => {
            each_annotation(scrutinee, f);
            for arm in arms {
                pattern_annotations(&arm.pat, f);
                each_annotation(&arm.body, f);
            }
        }
        ExprKind::Lambda(l) => {
            for p in &l.params {
                if let Some(t) = &p.ty {
                    f(t);
                }
            }
            if let Some(t) = &l.ret {
                f(t);
            }
            each_annotation(&l.body, f);
        }
    }
}

/// Calls `f` on every type argument written in pattern `p`.
fn pattern_annotations(p: &Pattern, f: &mut impl FnMut(&TypeExpr)) {
    match &p.kind {
        PatternKind::Move(_) => unreachable!("written by lift"),
        PatternKind::Variant(_, targs, _, pats) => {
            targs.iter().for_each(&mut *f);
            pats.iter().for_each(|p| pattern_annotations(p, f));
        }
        PatternKind::Struct(_, targs, fields, _) => {
            targs.iter().for_each(&mut *f);
            fields.iter().for_each(|fp| pattern_annotations(&fp.pat, f));
        }
        PatternKind::Or(alts) => alts.iter().for_each(|p| pattern_annotations(p, f)),
        PatternKind::Wild
        | PatternKind::Bind(..)
        | PatternKind::Int(..)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}
