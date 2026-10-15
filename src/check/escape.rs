//! Downward-only closures. A closure that captures variables keeps them in
//! the stack frame it was made in, so it must not outlive that frame. It can
//! be called, and passed down to other functions, but not returned, or stored
//! in a struct or enum. See docs/03-inference.md.
//!
//! Nothing in T can change a variable or reach the heap, so returning and
//! storing are the only ways a value can outlive the frame it's in. Calls and
//! fields are always safe: every function's result and every field were
//! checked here too.
//!
//! In a generic function, a value of type `T` isn't a function as far as
//! this can tell, though the caller may pick a function type. Each copy
//! monomorphisation makes is checked again, with `T` known, to catch that
//! (see `mono.rs`).

use super::Checker;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::Type;

/// Why a value may be a closure that has to stay where it was made.
#[derive(Debug, Clone)]
pub(super) enum Origin {
    /// A closure that captures this variable.
    Captures(String),
    /// This parameter, which may be any function the caller has, including
    /// a closure that captures.
    Param(String),
}

/// How a value would leave the frame it's in.
#[derive(Debug, Clone, Copy)]
pub(super) enum Exit {
    Return,
    Store,
}

/// A value that may be a closure that captures, leaving its frame. That's an
/// error if its type is a function type, which may not be known yet.
pub(super) struct Escape {
    ty: Type,
    span: Span,
    origin: Origin,
    exit: Exit,
}

/// The variables bound inside the expression being looked at, and what each
/// may hold, innermost last.
type Inner = Vec<(String, Option<Origin>)>;

impl Checker {
    /// Notes that `e`, of type `ty`, leaves the frame it's evaluated in by
    /// `exit`. If `e` may be a closure that captures, that's an error once
    /// `ty` is known to be a function type (see `check_escapes`).
    pub(super) fn leaves(&mut self, e: &Expr, ty: &Type, exit: Exit) {
        if let Some((origin, span)) = self.origin(e, &mut Vec::new()) {
            self.escapes.push(Escape {
                ty: ty.clone(),
                span,
                origin,
                exit,
            });
        }
    }

    /// At the end of a function, now that types are known: reports the first
    /// closure noted by `leaves` that does escape.
    pub(super) fn check_escapes(&mut self) -> Result<(), Error> {
        for escape in std::mem::take(&mut self.escapes) {
            // Only a function can capture. Other values that `origin` flags,
            // like an int parameter, are fine.
            if !matches!(self.shallow(&escape.ty), Type::Fn(..)) {
                continue;
            }
            let rule = "closures that capture can only be passed down";
            let message = match (escape.origin, escape.exit) {
                (Origin::Captures(x), Exit::Return) => format!(
                    "cannot return a closure that captures `{x}`: it keeps `{x}` on the \
                     stack, and {rule}, not returned"
                ),
                (Origin::Captures(x), Exit::Store) => format!(
                    "cannot store a closure that captures `{x}` in a struct or enum: it \
                     keeps `{x}` on the stack, and {rule}"
                ),
                (Origin::Param(p), Exit::Return) => format!(
                    "cannot return `{p}`: it may be a closure that captures, and {rule}, \
                     not returned"
                ),
                (Origin::Param(p), Exit::Store) => format!(
                    "cannot store `{p}` in a struct or enum: it may be a closure that \
                     captures, and {rule}"
                ),
            };
            return Err(Error::ty(escape.span, message));
        }
        Ok(())
    }

    /// Why `e` may evaluate to a closure that has to stay where it was made,
    /// if it may, and the part of `e` that makes it so. Only the shape of `e`
    /// matters, not its type: `leaves` filters by type later.
    ///
    /// `inner` holds the variables bound inside the outermost expression
    /// asked about, since those aren't in `self.locals`.
    pub(super) fn origin(&self, e: &Expr, inner: &mut Inner) -> Option<(Origin, Span)> {
        let is_local = |name: &str, inner: &Inner| {
            inner.iter().any(|(n, _)| n == name) || self.locals.iter().any(|l| l.name == name)
        };
        match &e.kind {
            ExprKind::Var(name, _) => {
                let origin = match inner.iter().rev().find(|(n, _)| n == name) {
                    Some((_, origin)) => origin.clone(),
                    None => self
                        .locals
                        .iter()
                        .rev()
                        .find(|l| l.name == *name)?
                        .origin
                        .clone(),
                };
                origin.map(|o| (o, e.span))
            }
            // Names that aren't locals are top-level functions, which are
            // always there.
            ExprKind::Lambda(l) => l
                .captures
                .iter()
                .find(|c| is_local(c, inner))
                .map(|c| (Origin::Captures(c.clone()), e.span)),
            ExprKind::If(_, then, els) => self
                .origin(then, inner)
                .or_else(|| els.as_ref().and_then(|els| self.origin(els, inner))),
            ExprKind::Block(stmts, tail) => {
                let scope = inner.len();
                for stmt in stmts {
                    if let Stmt::Let { pat, init, .. } = stmt {
                        let origin = self.origin(init, inner).map(|(o, _)| o);
                        bind(pat, origin, inner);
                    }
                }
                let found = tail.as_ref().and_then(|tail| self.origin(tail, inner));
                inner.truncate(scope);
                found
            }
            ExprKind::Match(scrutinee, arms) => {
                let origin = self.origin(scrutinee, inner).map(|(o, _)| o);
                arms.iter().find_map(|arm| {
                    let scope = inner.len();
                    bind(&arm.pat, origin.clone(), inner);
                    let found = self.origin(&arm.body, inner);
                    inner.truncate(scope);
                    found
                })
            }
            _ => None,
        }
    }
}

/// Adds the variables pattern `p` binds to `inner`, for a value that may be
/// a closure because of `origin`. Only a variable binding the whole value
/// can hold that closure: one inside a struct or variant gets a field, and
/// fields never hold one.
fn bind(p: &Pattern, origin: Option<Origin>, inner: &mut Inner) {
    match &p.kind {
        PatternKind::Bind(name) => inner.push((name.clone(), origin)),
        _ => inner.extend(p.binds().into_iter().map(|name| (name, None))),
    }
}

/// What a variable bound by pattern `p` may hold, given the `origin` of the
/// value matched: as in `bind`, only a variable binding the whole value.
pub(super) fn bound_origin(p: &Pattern, origin: Option<Origin>) -> Option<Origin> {
    match p.kind {
        PatternKind::Bind(_) => origin,
        _ => None,
    }
}
