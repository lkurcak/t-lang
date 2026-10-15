//! Which definition each name refers to. Locals are found on the stack of
//! `Checker::locals`, functions in the stack of `FnScope`s: one for the
//! top level, and one for each block that declares functions of its own.
//!
//! Each function has a *symbol*, a name for it that's unique in the whole
//! program, which the checker records for every use (`Info::symbols`). A
//! top-level function's symbol is its name. A nested one's is the name of
//! the one it's in and its own, like `main::helper`, with a number after if
//! that's taken (`main::helper#2`). No source program can write those, so
//! once checked, the nested functions can be moved to the top level under
//! them (see `lift.rs`), and nothing after that needs to know they were
//! ever nested.

use super::Checker;
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::types::Scheme;
use std::collections::{HashMap, HashSet};

/// A function, by where it is in `Checker::fns`: the top-level ones first,
/// in order, then the nested ones (see `Program::all_fns`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FnId(pub(super) usize);

/// A function's definition.
pub(super) struct FnDef {
    /// Its name, as written.
    pub(super) name: Ident,
    pub(super) symbol: String,
    pub(super) scheme: Scheme,
    /// The function it's nested in, if it is.
    pub(super) parent: Option<FnId>,
    /// Where its body is, for finding which function a use is in.
    pub(super) body: Span,
}

/// The functions a block declares, which are in scope in all of it. Locals
/// bound before the block (at indexes below `at` in `Checker::locals`) are
/// shadowed by them, and those bound in it shadow them.
pub(super) struct FnScope {
    pub(super) at: usize,
    pub(super) fns: Vec<FnId>,
}

/// What a name refers to, where it's used.
pub(super) enum Binding {
    /// The local at this index in `Checker::locals`.
    Local(usize),
    Fn(FnId),
    Builtin(&'static str),
}

/// A symbol for a function called `name` nested in the one with symbol
/// `parent`, that isn't in `taken`.
pub(super) fn symbol(parent: &str, name: &str, taken: &HashSet<String>) -> String {
    let symbol = format!("{parent}::{name}");
    (1..)
        .map(|n| match n {
            1 => symbol.clone(),
            n => format!("{symbol}#{n}"),
        })
        .find(|s| !taken.contains(s))
        .expect("there are always more numbers")
}

/// The name a function was written with, from its symbol or the name of a
/// copy of it that monomorphisation made: `helper` for `main::helper#2`,
/// `id::<int>` for `id::<int>`, `helper::<int>` for `main::helper::<int>`.
/// For messages, which shouldn't show symbols.
pub fn source_name(symbol: &str) -> String {
    let (base, args) = symbol.split_at(symbol.find("::<").unwrap_or(symbol.len()));
    let name = base.rsplit("::").next().unwrap_or(base);
    let name = name.split('#').next().unwrap_or(name);
    format!("{name}{args}")
}

/// The error for the first type in `t` that names a type parameter of a
/// function that nested `fn` `f` is in: one of `outer`, and not one of its
/// own `params`, or a struct or enum, if there is one. A nested `fn` means
/// what it would at the top level, so it can't see those.
pub(super) fn outer_param(
    t: &TypeExpr,
    f: &str,
    params: &[String],
    outer: &[String],
    arities: &HashMap<String, usize>,
) -> Option<Error> {
    match &t.kind {
        TypeExprKind::Named(name, args) => {
            if outer.contains(name) && !params.contains(name) && !arities.contains_key(name) {
                return Some(Error::ty(
                    t.span,
                    format!(
                        "can't use type parameter `{name}` of the enclosing function in a \
                         nested `fn`; declare it on this one too, as in `fn {f}<{name}>`"
                    ),
                ));
            }
            args.iter()
                .find_map(|a| outer_param(a, f, params, outer, arities))
        }
        TypeExprKind::Unit => None,
        TypeExprKind::Fn(ps, r) => ps
            .iter()
            .chain([&**r])
            .find_map(|p| outer_param(p, f, params, outer, arities)),
    }
}

impl Checker {
    /// What `name` refers to here, if anything: the innermost local or
    /// function by that name, or else a built-in.
    pub(super) fn binding(&self, name: &str) -> Option<Binding> {
        let local = self.locals.iter().rposition(|l| l.name == name);
        let func = self.scopes.iter().rev().find_map(|s| {
            let id = s.fns.iter().find(|id| self.fns[id.0].name.name == name)?;
            Some((s.at, *id))
        });
        match (local, func) {
            (Some(i), Some((at, id))) if at > i => Some(Binding::Fn(id)),
            (Some(i), _) => Some(Binding::Local(i)),
            (None, Some((_, id))) => Some(Binding::Fn(id)),
            (None, None) => super::BUILTINS
                .iter()
                .find(|&&b| b == name)
                .map(|&b| Binding::Builtin(b)),
        }
    }

    /// The error for a nested `fn` using `local`, at `span`, a local of a
    /// function it's in. The fix makes it a closure, if that fits (see
    /// `to_closure`), and if `local` is from the function right around it:
    /// otherwise the closure would be in a nested `fn` itself.
    pub(super) fn capture(&self, local: usize, span: Span) -> Error {
        let name = &self.locals[local].name;
        let e = Error::ty(
            span,
            format!("can't capture `{name}` in a nested `fn`; use a closure"),
        );
        match &self.closure_fix {
            Some((from, fix)) if local >= *from => e.with_fix(fix.clone()),
            _ => e,
        }
    }
}

/// The fix for nested `fn` `f`, statement `k` of a block with statements
/// `stmts` and result `tail`, using a local it can't: making it a closure,
/// which can. That only works if nothing a closure can't do is asked of it:
/// it isn't generic or recursive, nothing uses it before it (a `let` is
/// only in scope after it) or in another nested `fn` (which can't capture
/// it either), and every use calls it, since a closure that captures can't
/// be returned or stored. Its doc comment becomes a plain comment.
pub(super) fn to_closure(f: &FnDecl, stmts: &[Stmt], k: usize, tail: Option<&Expr>) -> Option<Fix> {
    let name = &f.name.name;
    let fits = f.type_params.is_empty()
        && !f.body.mentions(name)
        && stmts.iter().enumerate().all(|(j, s)| match s {
            Stmt::Fn(g) => j == k || !g.body.mentions(name),
            Stmt::Let { init: e, .. } | Stmt::Expr(e) | Stmt::BlockLike(e) => match j < k {
                true => !e.mentions(name),
                false => only_called(e, name),
            },
        })
        && tail.is_none_or(|t| only_called(t, name));
    if !fits {
        return None;
    }
    let mut edits: Vec<Edit> = f
        .doc
        .iter()
        .flat_map(|d| &d.lines)
        .map(|l| Edit::delete(Span::new(l.span.start + 2, l.span.start + 3)))
        .collect();
    edits.extend([
        Edit::replace(
            Span::new(f.span.start, f.parens.start + 1),
            format!("let {name} = |"),
        ),
        Edit::replace(Span::new(f.parens.end - 1, f.parens.end), "|"),
        Edit::insert(f.body.span.end, ";"),
    ]);
    Some(Fix::new(format!("make `{name}` a closure"), edits))
}

/// Is every use of `name` in `e` a call, outside any nested `fn`?
fn only_called(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Var(n, _) => n != name,
        ExprKind::Call(callee, args) if matches!(&callee.kind, ExprKind::Var(n, _) if n == name) => {
            args.iter().all(|a| only_called(a, name))
        }
        ExprKind::Block(stmts, _)
            if stmts
                .iter()
                .filter_map(Stmt::as_fn)
                .any(|g| g.body.mentions(name)) =>
        {
            false
        }
        _ => e.children().iter().all(|c| only_called(c, name)),
    }
}

/// What the checker was doing in the function a nested `fn` is in, put
/// aside while it checks the nested one.
pub(super) struct Outer {
    current: Option<FnId>,
    type_params: Vec<String>,
    barrier: usize,
    enclosing_params: usize,
    comparisons: Vec<(crate::types::Type, Span, BinOp)>,
    escapes: Vec<super::escape::Escape>,
    closure_fix: Option<(usize, Fix)>,
}

impl Checker {
    /// Checks nested `fn` `f` where it's declared, with `fix` for if it
    /// captures. It's a function of its own, so it's checked as one: with
    /// its own type parameters, and its own type variables and checks that
    /// wait for them, which must all be done with by its end. The locals
    /// around it stay on the stack, below `barrier`, to say what's wrong if
    /// it uses one.
    pub(super) fn nested(&mut self, f: &FnDecl, fix: Option<Fix>) -> Result<(), Error> {
        let id = self.nested_ids[&f.name.span];
        let barrier = self.locals.len();
        let outer = Outer {
            current: self.current,
            type_params: std::mem::take(&mut self.type_params),
            barrier: std::mem::replace(&mut self.barrier, barrier),
            enclosing_params: self.enclosing_params.len(),
            comparisons: std::mem::take(&mut self.comparisons),
            escapes: std::mem::take(&mut self.escapes),
            closure_fix: std::mem::take(&mut self.closure_fix),
        };
        self.closure_fix = fix.map(|fix| (outer.barrier, fix));
        self.enclosing_params
            .extend(outer.type_params.iter().cloned());
        let result = self.body(f, id);
        self.current = outer.current;
        self.type_params = outer.type_params;
        self.barrier = outer.barrier;
        self.enclosing_params.truncate(outer.enclosing_params);
        self.comparisons = outer.comparisons;
        self.escapes = outer.escapes;
        self.closure_fix = outer.closure_fix;
        result
    }
}
