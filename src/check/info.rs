//! What the checker learns about each name in a program, kept for later:
//! the type of every variable and use, what each name refers to, and which
//! type arguments each use of something generic has. Editors show it (hover,
//! inlay hints, go to definition, see `lsp.rs`), and monomorphisation reads
//! the type arguments (see `mono.rs`).

use super::Checker;
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
}

/// A use of something generic, with its type arguments: `id` in `id(1)` is
/// `Fn("id", [int])`.
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
    /// use of something generic (`::<int>`).
    pub fn hint(&self) -> Option<String> {
        if !self.inferred {
            return None;
        }
        match self.kind {
            NoteKind::Fn(_) | NoteKind::Struct(_) | NoteKind::Enum(_) => {
                Some(format!("::<{}>", comma_list(&self.args)))
            }
            _ => self.ty.as_ref().map(|t| format!(": {t}")),
        }
    }
}

/// Where each function, type, field and variant is declared, by name.
#[derive(Debug, Default)]
pub(super) struct Defs {
    pub(super) fns: HashMap<String, Span>,
    pub(super) types: HashMap<String, Span>,
    /// By `(type, field)`.
    pub(super) fields: HashMap<(String, String), Span>,
    /// By `(enum, variant)`.
    pub(super) variants: HashMap<(String, String), Span>,
}

impl Defs {
    /// The first declaration of each name: the checker rejects the others.
    pub(super) fn new(prog: &Program) -> Defs {
        let mut defs = Defs::default();
        for f in &prog.fns {
            defs.fns.entry(f.name.name.clone()).or_insert(f.name.span);
        }
        for d in &prog.types {
            let ty = d.name().name.clone();
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
            inferred: false,
        });
    }

    /// Like `note`, for a use of something generic with type arguments
    /// `args`, which monomorphisation will need. Uses of things that
    /// aren't generic need nothing. The type arguments were `inferred`
    /// unless they were written out.
    pub(super) fn note_use(
        &mut self,
        span: Span,
        kind: NoteKind,
        ty: Type,
        def: Option<Span>,
        instance: Option<Instance>,
        inferred: bool,
    ) {
        let args = match &instance {
            Some(Instance::Fn(_, args) | Instance::Adt(_, args)) => args.clone(),
            None => Vec::new(),
        };
        if let Some(instance) = instance
            && !args.is_empty()
        {
            self.pending.push((span, instance));
        }
        self.info.notes.push(Note {
            span,
            kind,
            ty: Some(ty),
            inferred: inferred && !args.is_empty(),
            args,
            def,
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
        for d in &prog.types {
            let params = d.type_params();
            let name = &d.name().name;
            let generic = Type::Adt(
                name.clone(),
                params.iter().map(|p| Type::Param(p.name.clone())).collect(),
            );
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
                        let ty = super::generics::resolve(&f.ty, &self.arities, &names(params));
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
                            .map(|t| super::generics::resolve(t, &self.arities, &names(params)))
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
        for f in &prog.fns {
            let params = &f.type_params;
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
                if let Some(p) = params.iter().find(|p| p.name == *name) {
                    let kind = NoteKind::TypeParam(name.clone());
                    self.note(span, kind, None, Some(p.span));
                } else if let Some(def) = self.defs.types.get(name).copied() {
                    let ty = super::generics::resolve(t, &self.arities, &names(params));
                    let kind = match self.adts.get(name).map(|d| &d.kind) {
                        Some(crate::types::AdtKind::Enum(_)) => NoteKind::Enum(name.clone()),
                        _ => NoteKind::Struct(name.clone()),
                    };
                    self.note(span, kind, ty.ok(), Some(def));
                }
                for a in args {
                    self.type_expr_notes(a, params);
                }
            }
            TypeExprKind::Unit => {}
            TypeExprKind::Fn(ps, r) => {
                for p in ps {
                    self.type_expr_notes(p, params);
                }
                self.type_expr_notes(r, params);
            }
        }
    }
}

fn names(params: &[Ident]) -> Vec<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

/// Calls `f` on every type written inside `e`: annotations on `let`s and
/// closures, and type arguments, in expressions and in patterns.
fn each_annotation(e: &Expr, f: &mut impl FnMut(&TypeExpr)) {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => {}
        ExprKind::Var(_, targs) => targs.iter().for_each(&mut *f),
        ExprKind::Unary(_, x) | ExprKind::Field(x, _) => each_annotation(x, f),
        ExprKind::Binary(_, l, r) => {
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
        | PatternKind::Bind(_)
        | PatternKind::Int(_)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}
