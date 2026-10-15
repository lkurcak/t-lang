//! What the checker learns about each name in a program, kept for later:
//! the type of every variable and use, what each name refers to and its doc
//! comment, and which type arguments each use of something generic has.
//! Editors show it (hover, inlay hints, go to definition, references and
//! rename, see `lsp.rs` and `rename.rs`), and monomorphisation reads the
//! type arguments (see `mono.rs`).

use super::{Checker, Lifetimes};
use crate::ast::*;
use crate::error::Span;
use crate::types::{Type, comma_list};
use std::collections::HashMap;

/// Everything the checker found out, as far as it got.
#[derive(Debug, Clone, Default)]
pub struct Info {
    /// One per name in the program that means something: variables where
    /// they're bound and where they're used, functions, types, fields and
    /// variants. In no particular order.
    pub notes: Vec<Note>,
    /// The type arguments of each use of a generic function, struct or enum,
    /// by the span of its name. In a generic function these can mention its
    /// own type parameters.
    pub instances: HashMap<Span, Instance>,
    /// The symbol of every function, struct and enum, by the span of its
    /// name where it's declared and where it's used: a name for it that's
    /// unique in the program, unlike the one it's written with (see
    /// `check/scope.rs` and `check/names.rs`).
    pub symbols: HashMap<Span, String>,
    /// Where each struct and enum is declared, by symbol: the span of its
    /// name.
    pub types: HashMap<String, Span>,
    /// The references the checker made into others, by the span of the
    /// expression: `r` used as a `&T` when it's a `&Box<T>`, a `&&T` or a
    /// `&mut T`, with how many more `*`s it takes to get to the `T`, and
    /// whether the result is `&mut`. `lift` writes them out: `&**r`.
    pub coercions: HashMap<Span, (usize, bool)>,
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
/// `Fn("id", [int])`. Each is named by its symbol.
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
    /// `let` without an annotation, say, or `id(1)` without `::<int>`.
    pub inferred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteKind {
    Var(String),
    Fn(String),
    Struct(String),
    Enum(String),
    TypeParam(String),
    Field(String),
    /// Enum, variant.
    Variant(String, String),
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
            NoteKind::Struct(_) => format!("struct {ty}"),
            NoteKind::Enum(_) => format!("enum {ty}"),
            NoteKind::TypeParam(name) => format!("type parameter {name}"),
            NoteKind::Field(name) => format!("field {name}: {ty}"),
            NoteKind::Variant(e, v) => format!("{e}{args}::{v}: {ty}"),
        }
    }

    /// What an editor might show right after it, if anything: the type of a
    /// variable whose type was inferred (`: int`), or the type arguments of a
    /// use of something generic (`::<int>`). Without lifetimes: inferred
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
#[derive(Debug, Default)]
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
    /// The first declaration of each: the checker rejects the others.
    /// `types` are those of `Program::all_types`, in order.
    pub(super) fn new(prog: &Program, types: &[super::TypeDef]) -> Defs {
        let mut defs = Defs::default();
        let mut doc = |name: &Ident, doc: Option<&DocComment>| {
            if let Some(d) = doc {
                defs.docs.insert(name.span, d.markdown());
            }
        };
        for (f, _) in prog.all_fns() {
            doc(&f.name, f.doc.as_ref());
        }
        let all = prog.all_types();
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
        defs
    }
}

impl Checker {
    /// Notes what's at `span`. Its types may still have variables in them:
    /// `finish_notes` solves them once the function is checked.
    pub(super) fn note(&mut self, span: Span, kind: NoteKind, ty: Option<Type>, def: Option<Span>) {
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
    /// it's generic, which monomorphisation will need, by the span of the
    /// whole use: `id::<int>` rather than just `id`. Uses of things that
    /// aren't generic need nothing. The type arguments were `inferred`
    /// unless they were written out.
    pub(super) fn note_use(
        &mut self,
        span: Span,
        kind: NoteKind,
        ty: Type,
        def: Option<Span>,
        instance: Option<(Span, Instance)>,
        inferred: bool,
    ) {
        let args = match &instance {
            Some((_, Instance::Fn(_, args) | Instance::Adt(_, args))) => args.clone(),
            None => Vec::new(),
        };
        if let Some(instance) = instance
            && !args.is_empty()
        {
            self.pending.push(instance);
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
                self.info.notes.push(Note {
                    span: site.span,
                    kind: NoteKind::Var(l.name.clone()),
                    ty: Some(l.ty.clone()),
                    args: Vec::new(),
                    def: Some(l.sites[0].span),
                    doc: None,
                    // `P { x }` binds `x`, but `P { x: int }` would read as
                    // a field holding the pattern `int`.
                    inferred: inferred && !site.shorthand,
                });
            }
        }
    }

    /// At the end of a function, checked or not: solves the variables in
    /// the notes made since `notes` and in the pending instances, as far as
    /// they're known, and files the instances in `info`.
    pub(super) fn finish_notes(&mut self, notes: usize) {
        for i in notes..self.info.notes.len() {
            let n = &self.info.notes[i];
            let ty = n.ty.as_ref().map(|t| self.solved(t));
            let args = n.args.iter().map(|t| self.solved(t)).collect();
            let n = &mut self.info.notes[i];
            n.ty = ty;
            n.args = args;
        }
        for (span, instance) in std::mem::take(&mut self.pending) {
            let instance = match instance {
                Instance::Fn(name, args) => {
                    Instance::Fn(name, args.iter().map(|t| self.solved(t)).collect())
                }
                Instance::Adt(name, args) => {
                    Instance::Adt(name, args.iter().map(|t| self.solved(t)).collect())
                }
            };
            self.info.instances.insert(span, instance);
        }
    }

    /// Notes every type written in the program, and every declaration of a
    /// type, type parameter, field or variant. None of that needs inference,
    /// so it's done for the whole program, even past an error.
    pub(super) fn type_notes(&mut self, prog: &Program) {
        for (i, (d, _)) in prog.all_types().into_iter().enumerate() {
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
            self.param_notes(params);
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
        for (f, _) in prog.all_fns() {
            let params = &f.generics.types;
            self.param_notes(params);
            for p in &f.params {
                self.type_expr_notes(&p.ty, params);
            }
            self.type_expr_notes(&f.ret, params);
            each_annotation(&f.body, &mut |t| self.type_expr_notes(t, params));
        }
    }

    fn param_notes(&mut self, params: &[Ident]) {
        for p in params {
            let kind = NoteKind::TypeParam(p.name.clone());
            self.note(p.span, kind, None, Some(p.span));
        }
    }

    /// Notes the names in type `t`, written where type parameters `params`
    /// are in scope.
    fn type_expr_notes(&mut self, t: &TypeExpr, params: &[Ident]) {
        match &t.kind {
            TypeExprKind::Named(name, args) => {
                // Only the name, not the arguments after it.
                let span = Span::new(t.span.start, t.span.start + name.len());
                if let Some(symbol) = self.adt_symbol(name, t.span) {
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
            TypeExprKind::Unit | TypeExprKind::Lifetime(_) => {}
            TypeExprKind::Fn(ps, r) => {
                for p in ps {
                    self.type_expr_notes(p, params);
                }
                self.type_expr_notes(r, params);
            }
            TypeExprKind::Ref(_, _, inner) => self.type_expr_notes(inner, params),
        }
    }
}

fn names(params: &[Ident]) -> Vec<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

/// Calls `f` on every type written inside `e`: annotations on `let`s and
/// closures, and type arguments, in expressions and in patterns. Not in
/// the functions nested in it.
fn each_annotation(e: &Expr, f: &mut impl FnMut(&TypeExpr)) {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => {}
        ExprKind::Var(_, targs) => targs.iter().for_each(&mut *f),
        ExprKind::Unary(_, x) | ExprKind::Field(x, _) => each_annotation(x, f),
        ExprKind::Binary(_, l, r) | ExprKind::Assign(_, l, r) | ExprKind::While(l, r) => {
            each_annotation(l, f);
            each_annotation(r, f);
        }
        ExprKind::Call(callee, args) => {
            each_annotation(callee, f);
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
                    Stmt::Let { pat, ty, init } => {
                        pattern_annotations(pat, f);
                        if let Some(t) = ty {
                            f(t);
                        }
                        each_annotation(init, f);
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
        ExprKind::Variant(_, targs, _, args) => {
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
        | PatternKind::Int(_)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}
