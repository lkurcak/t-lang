//! Which definition each name refers to. Locals are found on the stack of
//! `Checker::locals`, functions in the stack of `FnScope`s: one for the
//! top level, and one for each block that declares functions of its own.
//! Names in types are worked out before checking (see `check/names.rs`).
//!
//! Each function has a *symbol*, a name for it that's unique in the whole
//! program, which the checker records for every use (`Info::symbols`). A
//! top-level function's symbol is its name. A nested one's is the name of
//! the one it's in and its own, like `main::helper`, with a number after if
//! that's taken (`main::helper#2`). No source program can write those, so
//! once checked, the nested functions can be moved to the top level under
//! them (see `lift.rs`), and nothing after that needs to know they were
//! ever nested. Local types get symbols the same way.
//!
//! A method's symbol says what type and trait it's for: `<Point>::len` in
//! `impl Point`, `<Point as Show>::show` in `impl Show for Point`, and
//! `<Show>::show` for a trait's default, which is also what a call of the
//! trait's method names.

use super::Checker;
use super::modules::Modules;
use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::types::Scheme;
use std::collections::HashSet;

/// A function, by where it is in `Checker::fns`: the top-level ones first,
/// in order, then the nested ones (see `Program::all_fns`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FnId(pub(super) usize);

/// A function's definition.
#[derive(Clone)]
pub(super) struct FnDef {
    /// Its name, as written.
    pub(super) name: Ident,
    pub(super) symbol: String,
    pub(super) scheme: Scheme,
    /// The function it's nested in, if it is.
    pub(super) parent: Option<FnId>,
    /// Where its body is, for finding which function a use is in.
    pub(super) body: Span,
    /// The module it's in, and how far it's seen from there.
    pub(super) module: usize,
    pub(super) level: Level,
}

/// The functions a block declares, which are in scope in all of it. Locals
/// bound before the block (at indexes below `at` in `Checker::locals`) are
/// shadowed by them, and those bound in it shadow them.
#[derive(Clone)]
pub(super) struct FnScope {
    pub(super) at: usize,
    pub(super) fns: Vec<FnId>,
}

/// What a name refers to, where it's used.
pub(super) enum Binding {
    /// The local at this index in `Checker::locals`.
    Local(usize),
    Fn(FnId),
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

/// The symbol of each function of `prog`, in the order of
/// `Program::all_fns`: `before`, those of the packages before, then the
/// rest.
pub(super) fn symbols(prog: &Program, modules: &Modules, before: &[String]) -> Vec<String> {
    let mut out: Vec<String> = before.to_vec();
    let mut taken: HashSet<String> = before.iter().cloned().collect();
    for (f, parent, method) in prog.all_fns().into_iter().skip(before.len()) {
        let symbol = match (parent, method) {
            (Some(p), _) => symbol(&out[p], &f.name.name, &taken),
            (None, Some(MethodOf::Impl(k))) => {
                let d = &prog.impls[k];
                let header = match &d.trait_name {
                    Some(t) => format!("{} as {}", d.self_ty, t.name),
                    None => d.self_ty.to_string(),
                };
                let symbol = format!("<{header}>::{}", f.name.name);
                // Two impls can only have the same header by mistake, which
                // the checker rejects, or for types of the same name in
                // different modules.
                (1..)
                    .map(|n| match n {
                        1 => symbol.clone(),
                        n => format!("{symbol}#{n}"),
                    })
                    .find(|s| !taken.contains(s))
                    .expect("there are always more numbers")
            }
            (None, Some(MethodOf::Trait(k))) => {
                super::traits::trait_method_symbol(&prog.traits[k].name.name, &f.name.name)
            }
            (None, None) => format!("{}{}", modules.mods[f.module].prefix, f.name.name),
        };
        taken.insert(symbol.clone());
        out.push(symbol);
    }
    out
}

/// The name a function or type was written with, from its symbol or the
/// name of a copy of it that monomorphisation made: `helper` for
/// `main::helper#2`, `id::<i64>` for `id::<i64>`, `helper::<Point>` for
/// `main::helper::<main::Point>`. For messages, which shouldn't show
/// symbols. Anything else in `symbol`, like the `fn(..) -> ..` of a type,
/// stays as it is.
pub fn source_name(symbol: &str) -> String {
    // A method: `<Point>::len` is `Point::len`, and `<Point as Show>::show`
    // stays as it is. One nested in a method is only its own name.
    if let Some(rest) = symbol.strip_prefix('<')
        && let Some(end) = closing(rest)
        && let Some(method) = rest[end + 1..].strip_prefix("::")
    {
        let header = &rest[..end];
        let name = source_name(method);
        let own = method
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next();
        if !own.is_some_and(|own| name.starts_with(own)) {
            return name;
        }
        return match header.split_once(" as ") {
            Some((ty, tr)) => format!("<{} as {tr}>::{name}", source_name(ty)),
            None => format!("{}::{name}", source_name(header)),
        };
    }
    let mut out = String::new();
    let mut rest = symbol;
    while let Some(c) = rest.chars().next() {
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if len == 0 {
            out.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        }
        let (name, after) = rest.split_at(len);
        // The number that tells apart nested things of the same name.
        let after = match after.strip_prefix('#') {
            Some(n) => n.trim_start_matches(|c: char| c.is_ascii_digit()),
            None => after,
        };
        // A function's name before what's nested in it, rather than before
        // type arguments, is dropped.
        match after.strip_prefix("::") {
            Some(inner) if !inner.starts_with('<') => rest = inner,
            _ => {
                out.push_str(name);
                rest = after;
            }
        }
    }
    out
}

/// Where the `>` is that closes the `<` before `s`.
fn closing(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            // Not the `>` of a `->`.
            '>' if i > 0 && s.as_bytes()[i - 1] == b'-' => {}
            '>' if depth == 0 => return Some(i),
            '>' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Where the function or type with symbol `symbol` is declared, for telling
/// apart two of the same name: "in `main`", or "at the top level".
pub(super) fn declared_in(symbol: &str) -> String {
    let base = &symbol[..symbol.find("::<").unwrap_or(symbol.len())];
    match base.rsplit_once("::") {
        Some((parent, _)) => format!("in `{}`", source_name(parent)),
        None => "at the top level".to_string(),
    }
}

impl Checker {
    /// What `name` refers to here, if anything: the innermost local or
    /// function by that name, or else one of its module's.
    pub(super) fn binding(&self, name: &str) -> Option<Binding> {
        let local = self.locals.iter().rposition(|l| l.name == name);
        let func = self.scopes.iter().rev().find_map(|s| {
            let id = s.fns.iter().find(|id| self.fns[id.0].name.name == name)?;
            Some((s.at, *id))
        });
        // Its module's are in scope everywhere in it, so behind every local.
        let module = || Some((0, FnId(self.modules.fn_named(self.module, name)?)));
        let func = func.or_else(module);
        match (local, func) {
            (Some(i), Some((at, id))) if at > i => Some(Binding::Fn(id)),
            (Some(i), _) => Some(Binding::Local(i)),
            (None, Some((_, id))) => Some(Binding::Fn(id)),
            (None, None) => None,
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
    let fits = f.generics.is_empty()
        && !f.body.mentions(name)
        && stmts.iter().enumerate().all(|(j, s)| match s {
            Stmt::Fn(g) => j == k || !g.body.mentions(name),
            Stmt::Type(_) => true,
            Stmt::Let { init: None, .. } => true,
            Stmt::Let { init: Some(e), .. } | Stmt::Expr(e) | Stmt::BlockLike(e) => match j < k {
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
    type_params: crate::types::Params,
    const_decls: Vec<crate::ast::Ident>,
    lifetimes: Vec<String>,
    barrier: usize,
    env: Vec<crate::traits::Pred>,
    self_ty: Option<crate::types::Type>,
    obligations: Vec<super::traits::Obligation>,
    closures: Vec<super::closures::Pending>,
    calls: Vec<(crate::ast::NodeId, crate::types::Type, Span)>,
    unused: Vec<(usize, crate::types::Type)>,
    closure_fix: Option<(usize, Fix)>,
    ret: Option<crate::types::Type>,
    loops: Vec<super::Target>,
    diverges: Option<Span>,
    diverging: Vec<crate::types::Type>,
    effect_sites: Vec<super::effects::Site>,
    unwraps: Vec<(super::lints::Unwrap, crate::types::Type)>,
    dropped: Vec<(Span, crate::types::Type)>,
    in_trait_impl: bool,
    withins: Vec<super::effects::Within>,
    unsafe_blocks: Vec<Span>,
    unsafe_uses: Vec<(Span, crate::types::Type)>,
    literals: Vec<super::Literal>,
    nots: Vec<(crate::types::Type, Span)>,
}

impl Checker {
    /// Checks nested `fn` `f` where it's declared, with `fix` for if it
    /// captures. It's a function of its own, so it's checked as one: with
    /// its own type parameters, and its own type variables and checks that
    /// wait for them, which must all be done with by its end. The locals
    /// around it stay on the stack, below `barrier`, to say what's wrong if
    /// it uses one.
    pub(super) fn nested(&mut self, f: &FnDecl, fix: Option<Fix>) -> Result<(), Error> {
        let id = self.nested_ids[&f.name.id];
        let barrier = self.locals.len();
        let outer = Outer {
            current: self.current,
            type_params: std::mem::take(&mut self.type_params),
            const_decls: std::mem::take(&mut self.const_decls),
            lifetimes: std::mem::take(&mut self.lifetimes),
            barrier: std::mem::replace(&mut self.barrier, barrier),
            env: std::mem::take(&mut self.env),
            self_ty: self.self_ty.take(),
            obligations: std::mem::take(&mut self.obligations),
            closures: std::mem::take(&mut self.closures),
            calls: std::mem::take(&mut self.calls),
            unused: std::mem::take(&mut self.unused),
            closure_fix: std::mem::take(&mut self.closure_fix),
            ret: self.ret.take(),
            loops: std::mem::take(&mut self.loops),
            diverges: self.diverges.take(),
            diverging: std::mem::take(&mut self.diverging),
            effect_sites: std::mem::take(&mut self.effect_sites),
            unwraps: std::mem::take(&mut self.unwraps),
            dropped: std::mem::take(&mut self.dropped),
            in_trait_impl: std::mem::take(&mut self.in_trait_impl),
            withins: std::mem::take(&mut self.withins),
            unsafe_blocks: std::mem::take(&mut self.unsafe_blocks),
            unsafe_uses: std::mem::take(&mut self.unsafe_uses),
            literals: std::mem::take(&mut self.literals),
            nots: std::mem::take(&mut self.nots),
        };
        self.closure_fix = fix.map(|fix| (outer.barrier, fix));
        let result = self.body(f, id);
        self.current = outer.current;
        self.type_params = outer.type_params;
        self.const_decls = outer.const_decls;
        self.lifetimes = outer.lifetimes;
        self.barrier = outer.barrier;
        self.env = outer.env;
        self.self_ty = outer.self_ty;
        self.obligations = outer.obligations;
        self.closures = outer.closures;
        self.calls = outer.calls;
        self.unused = outer.unused;
        self.closure_fix = outer.closure_fix;
        self.ret = outer.ret;
        self.loops = outer.loops;
        self.diverges = outer.diverges;
        self.diverging = outer.diverging;
        self.effect_sites = outer.effect_sites;
        self.unwraps = outer.unwraps;
        self.dropped = outer.dropped;
        self.in_trait_impl = outer.in_trait_impl;
        self.withins = outer.withins;
        self.unsafe_blocks = outer.unsafe_blocks;
        self.unsafe_uses = outer.unsafe_uses;
        self.literals = outer.literals;
        self.nots = outer.nots;
        result
    }
}
