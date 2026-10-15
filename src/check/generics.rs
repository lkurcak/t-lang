//! Type parameters: resolving the names in written types against them, and
//! the rules that keep generic code monomorphisable (see
//! docs/04-generics.md).

use super::scope::source_name;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::Type;
use std::collections::{HashMap, HashSet};

/// "1 type argument", "2 type arguments".
fn type_args(n: usize) -> String {
    format!("{n} type argument{}", if n == 1 { "" } else { "s" })
}

/// `what` takes `arity` type arguments, but `given` were written at `span`.
pub(super) fn arity_error(what: &str, arity: usize, given: usize, span: Span) -> Error {
    let message = match arity {
        0 => format!("{what} takes no type arguments"),
        n => format!(
            "{what} takes {} but {given} {} supplied",
            type_args(n),
            if given == 1 { "was" } else { "were" }
        ),
    };
    Error::ty(span, message)
}

/// What a name in a type refers to, if it isn't a type parameter of what
/// it's written in, or `int` or `bool`.
pub(crate) enum TypeName {
    /// A struct or enum, by its symbol (see `check/names.rs`), with how
    /// many type arguments it takes.
    Adt(String, usize),
    /// A type parameter of a function that what it's written in is nested
    /// in, which it can't use. With the keyword and name of what it's
    /// written in: `("fn", "helper")`.
    OuterParam(&'static str, String),
}

/// Turns a type as written (`TypeExpr`) into a type as understood (`Type`).
/// `names` says what the name at a span refers to, and `params` are the type
/// parameters in scope, for names it says nothing about.
pub(crate) fn resolve(
    t: &TypeExpr,
    names: &dyn Fn(&str, Span) -> Option<TypeName>,
    params: &[String],
) -> Result<Type, Error> {
    match &t.kind {
        TypeExprKind::Named(name, _) if name == "_" => Err(Error::ty(
            t.span,
            "`_` can only stand for a type in type arguments, like `id::<_>`",
        )),
        TypeExprKind::Named(name, args) => {
            let (ty, arity) = match names(name, t.span) {
                Some(TypeName::Adt(symbol, n)) => (Type::Adt(symbol, Vec::new()), n),
                Some(TypeName::OuterParam(keyword, item)) => {
                    let nested = if keyword == "fn" { "nested" } else { "local" };
                    return Err(Error::ty(
                        t.span,
                        format!(
                            "can't use type parameter `{name}` of the enclosing function in a \
                             {nested} `{keyword}`; declare it on this one too, as in \
                             `{keyword} {item}<{name}>`"
                        ),
                    ));
                }
                None if params.contains(name) => (Type::Param(name.clone()), 0),
                None if name == "int" => (Type::Int, 0),
                None if name == "bool" => (Type::Bool, 0),
                None => return Err(Error::ty(t.span, format!("unknown type `{name}`"))),
            };
            if args.len() != arity {
                let what = match ty {
                    Type::Param(_) => format!("type parameter `{name}`"),
                    _ => format!("`{name}`"),
                };
                return Err(arity_error(&what, arity, args.len(), t.span));
            }
            let args = args
                .iter()
                .map(|a| resolve(a, names, params))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(match ty {
                Type::Adt(symbol, _) => Type::Adt(symbol, args),
                ty => ty,
            })
        }
        TypeExprKind::Unit => Ok(Type::Unit),
        TypeExprKind::Fn(params_, ret) => Ok(Type::func(
            params_
                .iter()
                .map(|p| resolve(p, names, params))
                .collect::<Result<_, _>>()?,
            resolve(ret, names, params)?,
        )),
    }
}

/// `t`, with each `_` in it renamed to a type parameter no program can
/// name (`?0`, `?1`, ..), which are added to `holes` with where they were.
pub(super) fn name_holes(t: &TypeExpr, holes: &mut Vec<(String, Span)>) -> TypeExpr {
    let kind = match &t.kind {
        TypeExprKind::Named(name, args) if name == "_" && args.is_empty() => {
            let name = format!("?{}", holes.len());
            holes.push((name.clone(), t.span));
            TypeExprKind::Named(name, Vec::new())
        }
        TypeExprKind::Named(name, args) => TypeExprKind::Named(
            name.clone(),
            args.iter().map(|a| name_holes(a, holes)).collect(),
        ),
        TypeExprKind::Unit => TypeExprKind::Unit,
        TypeExprKind::Fn(params, ret) => TypeExprKind::Fn(
            params.iter().map(|p| name_holes(p, holes)).collect(),
            Box::new(name_holes(ret, holes)),
        ),
    };
    TypeExpr { kind, span: t.span }
}

/// Checks the type parameters declared by a function, struct or enum, and
/// returns their names.
pub(super) fn declare(params: &[Ident]) -> Result<Vec<String>, Error> {
    let mut names: Vec<String> = Vec::new();
    for p in params {
        if p.name == "_" {
            return Err(Error::ty(
                p.span,
                "a type parameter needs a name; `_` means \"infer this\" in type arguments",
            ));
        }
        if matches!(p.name.as_str(), "int" | "bool") {
            return Err(Error::ty(
                p.span,
                format!("`{}` is a built-in type", p.name),
            ));
        }
        if names.contains(&p.name) {
            return Err(Error::ty(
                p.span,
                format!("type parameter `{}` is declared twice", p.name),
            ));
        }
        names.push(p.name.clone());
    }
    Ok(names)
}

/// Does `t` mention type parameter `p`?
fn mentions(t: &Type, p: &str) -> bool {
    matches!(t, Type::Param(q) if q == p) || t.parts().any(|t| mentions(t, p))
}

/// One generic thing using another: function `from` calls or names
/// function `to`, or struct or enum `from` has a field that mentions `to`,
/// with type arguments `args`, which may mention `from`'s own parameters.
/// Functions are named by their symbols.
pub(super) struct Use<'a> {
    pub(super) from: &'a str,
    pub(super) to: &'a str,
    pub(super) args: &'a [Type],
    pub(super) span: Span,
}

/// Rejects polymorphic recursion that would make monomorphisation go on
/// forever, like `fn f<T>(x: T) { f(Option::Some(x)) }`: `f<int>` needs
/// `f<Option<int>>`, which needs `f<Option<Option<int>>>`, and so on.
///
/// Every copy monomorphisation makes is of some item with arguments built
/// from the arguments of the copy that needed it. So it only goes on
/// forever if some type parameter flows, through uses, back into itself
/// with something wrapped around it. That's a graph problem: there's a node
/// for each parameter of each item, and a use of `to<.., A, ..>` in `from`
/// gives an edge from each parameter of `from` that `A` mentions to `to`'s
/// parameter. The edge *grows* if `A` is more than just that parameter.
/// Monomorphisation finishes exactly when no cycle has an edge that grows.
/// (This is how .NET's ECMA-335 spec rules out "expansive" generic types.)
///
/// `params` holds each item's type parameters.
pub(super) fn polymorphic_recursion(
    uses: &[Use],
    params: &HashMap<&str, &[String]>,
) -> Result<(), Error> {
    // Edges between `(item, parameter index)` nodes, and whether each grows.
    type Node<'a> = (&'a str, usize);
    let mut edges: Vec<(Node, Node, bool, &Use)> = Vec::new();
    for u in uses {
        let Some(from_params) = params.get(u.from) else {
            continue;
        };
        for (j, arg) in u.args.iter().enumerate() {
            for (i, p) in from_params.iter().enumerate() {
                if mentions(arg, p) {
                    let grows = *arg != Type::Param(p.clone());
                    edges.push(((u.from, i), (u.to, j), grows, u));
                }
            }
        }
    }
    let reaches = |start: (&str, usize), goal: (&str, usize)| {
        let mut seen = HashSet::new();
        let mut todo = vec![start];
        while let Some(n) = todo.pop() {
            if n == goal {
                return true;
            }
            if seen.insert(n) {
                todo.extend(edges.iter().filter(|e| e.0 == n).map(|e| e.1));
            }
        }
        false
    };
    for &(a, b, grows, u) in &edges {
        if grows && reaches(b, a) {
            let arg = &u.args[b.1];
            let to_p = &params[u.to][b.1];
            let (from, to) = (source_name(u.from), source_name(u.to));
            let message = if u.from == u.to {
                format!(
                    "polymorphic recursion: `{from}` uses itself with `{to_p}` = `{arg}`, a bigger \
                     type each time round, so monomorphising it would never end"
                )
            } else {
                format!(
                    "polymorphic recursion: this use of `{to}` with `{to_p}` = `{arg}` leads \
                     back to `{from}`, with a bigger type each time round, so monomorphising \
                     them would never end"
                )
            };
            return Err(Error::ty(u.span, message));
        }
    }
    Ok(())
}
