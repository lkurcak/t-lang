//! Downward-only closures. A closure that captures variables keeps them in
//! the stack frame it was made in, so it must not outlive that frame. It can
//! be called, and passed down to other functions, but not returned, or stored
//! in a struct or enum. See docs/03-inference.md.
//!
//! Returning, storing in a struct, enum or box, and assigning are the only
//! ways a value can outlive the frame it's in. Calls and fields are always
//! safe: every function's result and every field were checked here too.
//! What a reference points to isn't: it may be a variable of a function
//! further up, holding a closure that captures from that function's frame,
//! so the closure can only be passed further down.
//!
//! In a generic function, a value of type `T` isn't a function as far as
//! this can tell, though the caller may pick a function type. Each copy
//! monomorphisation makes is checked again, with `T` known, to catch that
//! (see `mono.rs`).

use super::Checker;
use super::scope::Binding;
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
    /// What a reference points to, which may be a variable holding a
    /// closure that captures. With the reference's type: what a box points
    /// to is fine, since it was stored, so checked.
    Deref(Type),
}

/// How a value would leave the frame it's in.
#[derive(Debug, Clone, Copy)]
pub(super) enum Exit {
    Return,
    Store,
    Assign,
}

/// A value that may be a closure that captures, leaving its frame. That's an
/// error if its type is a function type, which may not be known yet.
pub(super) struct Escape {
    ty: Type,
    span: Span,
    origin: Origin,
    exit: Exit,
}

/// The names bound inside the expression being looked at, innermost last:
/// `Some` of what it may hold for a variable, `None` for a nested function.
type Inner = Vec<(String, Option<Option<Origin>>)>;

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
            if let Origin::Deref(t) = &escape.origin
                && let Type::Box(_) = self.shallow(t)
            {
                continue;
            }
            let rule = "closures that capture can only be passed down";
            let message = match (escape.origin, escape.exit) {
                (Origin::Captures(x), Exit::Assign) => format!(
                    "cannot assign a closure that captures `{x}`: it keeps `{x}` on the stack, \
                     and {rule}"
                ),
                (Origin::Param(p), Exit::Assign) => {
                    format!("cannot assign `{p}`: it may be a closure that captures, and {rule}")
                }
                (Origin::Deref(_), exit) => {
                    let what = match exit {
                        Exit::Return => "return",
                        Exit::Store => "store",
                        Exit::Assign => "assign",
                    };
                    format!(
                        "cannot {what} this function: it's read through a reference, so it may \
                         be a closure that captures, and {rule}"
                    )
                }
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
        // `Some` of what `name` may hold if it's a variable, bound inside
        // `e` or around it.
        let local = |name: &str, inner: &Inner| match inner.iter().rev().find(|(n, _)| n == name) {
            Some((_, local)) => local.clone(),
            None => match self.binding(name) {
                Some(Binding::Local(i)) => Some(self.locals[i].origin.clone()),
                _ => None,
            },
        };
        match &e.kind {
            ExprKind::Var(name, _) => local(name, inner).flatten().map(|o| (o, e.span)),
            ExprKind::Unary(UnOp::Deref, x) => {
                let t = self.typed.exprs.get(&std::ptr::from_ref(&**x)).cloned();
                Some((Origin::Deref(t.unwrap_or(Type::Unit)), e.span))
            }
            // Names that aren't locals are functions, which are always
            // there.
            ExprKind::Lambda(l) => l
                .captures
                .iter()
                .find(|c| local(c, inner).is_some())
                .map(|c| (Origin::Captures(c.clone()), e.span)),
            ExprKind::If(_, then, els) => self
                .origin(then, inner)
                .or_else(|| els.as_ref().and_then(|els| self.origin(els, inner))),
            ExprKind::Block(stmts, tail) => {
                let scope = inner.len();
                let fns = stmts.iter().filter_map(Stmt::as_fn);
                inner.extend(fns.map(|f| (f.name.name.clone(), None)));
                for stmt in stmts {
                    if let Stmt::Let { pat, init, .. } = stmt {
                        let origin = init.as_ref().and_then(|e| self.origin(e, inner));
                        bind(pat, origin.map(|(o, _)| o), inner);
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
        PatternKind::Bind(name, _) => inner.push((name.clone(), Some(origin))),
        _ => inner.extend(p.binds().into_iter().map(|name| (name, Some(None)))),
    }
}

/// What a variable bound by pattern `p` may hold, given the `origin` of the
/// value matched: as in `bind`, only a variable binding the whole value.
pub(super) fn bound_origin(p: &Pattern, origin: Option<Origin>) -> Option<Origin> {
    match p.kind {
        PatternKind::Bind(..) => origin,
        _ => None,
    }
}
