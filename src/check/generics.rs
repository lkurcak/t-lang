//! Type parameters: resolving the names in written types against them, and
//! the rules that keep generic code monomorphisable (see
//! docs/04-generics.md).

use super::scope::source_name;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{FnType, Region, Type};
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
    /// many lifetime and type arguments it takes.
    Adt(String, usize, usize),
    /// A type parameter of a function that what it's written in is nested
    /// in, which it can't use. With the keyword and name of what it's
    /// written in: `("fn", "helper")`.
    OuterParam(&'static str, String),
}

/// What the lifetimes written in a type mean, which depends on where it's
/// written.
pub(crate) enum Lifetimes<'a> {
    /// In a function's parameters, where `declared` are its lifetime
    /// parameters. Each lifetime left out gets a parameter of its own,
    /// named `'_0`, `'_1` and so on, added to `elided`. That's *lifetime
    /// elision*.
    Params {
        declared: &'a [String],
        elided: Vec<String>,
    },
    /// In a function's return type. A lifetime left out is `input`'s, if
    /// it's there: the one lifetime the parameters have.
    Return {
        declared: &'a [String],
        input: Result<Region, usize>,
    },
    /// In a function's body: one left out is for the borrow checker to work
    /// out.
    Body { declared: &'a [String] },
    /// In the fields of a struct or enum, where none can be left out.
    Fields { declared: &'a [String] },
    /// Anywhere, as written: in what monomorphisation made, and where types
    /// are only being shown.
    Any,
}

impl Lifetimes<'_> {
    /// The region lifetime `l` names, or the one it stands for if it's left
    /// out (`None`), in a type written at `span`.
    pub(crate) fn region(&mut self, l: Option<&str>, span: Span) -> Result<Region, Error> {
        let declared = match self {
            Lifetimes::Params { declared, .. }
            | Lifetimes::Return { declared, .. }
            | Lifetimes::Body { declared }
            | Lifetimes::Fields { declared } => *declared,
            Lifetimes::Any => &[],
        };
        match l {
            Some("'static") => return Ok(Region::Static),
            Some(name) if name != "'_" => {
                if matches!(self, Lifetimes::Any) || declared.iter().any(|d| d == name) {
                    return Ok(Region::Named(name.to_string()));
                }
                return Err(Error::ty(
                    span,
                    format!(
                        "use of undeclared lifetime `{name}`; declare it, as in `fn f<{name}>`"
                    ),
                ));
            }
            _ => {}
        }
        match self {
            Lifetimes::Params { elided, .. } => {
                let name = format!("'_{}", elided.len());
                elided.push(name.clone());
                Ok(Region::Named(name))
            }
            Lifetimes::Return { input: Ok(r), .. } => Ok(r.clone()),
            Lifetimes::Return { input: Err(0), .. } => Err(Error::ty(
                span,
                "missing lifetime: this returns a reference, but there's nothing it could \
                 borrow from, as no parameter holds one",
            )),
            Lifetimes::Return { input: Err(n), .. } => Err(Error::ty(
                span,
                format!(
                    "missing lifetime: this returns a reference, but the parameters have {n} \
                     lifetimes, so it's not clear which it borrows from; name the one it \
                     does, as in `fn f<'a>(x: &'a int, y: &int) -> &'a int`"
                ),
            )),
            Lifetimes::Body { .. } | Lifetimes::Any => Ok(Region::Erased),
            Lifetimes::Fields { .. } => Err(Error::ty(
                span,
                "missing lifetime: a reference in a struct or enum needs one, as in \
                 `struct S<'a> { r: &'a int }`",
            )),
        }
    }
}

/// Turns a type as written (`TypeExpr`) into a type as understood (`Type`).
/// `names` says what the name at a span refers to, and `params` are the type
/// parameters in scope, for names it says nothing about. `lifetimes` says
/// what lifetimes mean.
pub(crate) fn resolve(
    t: &TypeExpr,
    names: &dyn Fn(&str, Span) -> Option<TypeName>,
    params: &[String],
    lifetimes: &mut Lifetimes,
) -> Result<Type, Error> {
    resolve_in(t, names, params, lifetimes, None)
}

/// The lifetimes a function type being resolved is generic over: those
/// left out in its parameters. In its result, one left out is the one
/// lifetime its parameters have, if they have exactly one.
struct Binder {
    bound: Vec<String>,
    /// Once its parameters are resolved, their lifetime, or how many they
    /// have if it isn't one.
    ret: Option<Result<Region, usize>>,
}

/// The region lifetime `l` names, written at `span` in a type, or the one
/// it stands for if it's left out, inside the function type `binder` is
/// for, if it's in one.
fn region(
    l: Option<&str>,
    span: Span,
    lifetimes: &mut Lifetimes,
    binder: Option<&mut Binder>,
) -> Result<Region, Error> {
    match (l, binder) {
        (Some(name), _) if name != "'_" => lifetimes.region(Some(name), span),
        (_, Some(b)) => match &b.ret {
            None => {
                let name = format!("'^{}", b.bound.len());
                b.bound.push(name.clone());
                Ok(Region::Named(name))
            }
            Some(Ok(r)) => Ok(r.clone()),
            Some(Err(n)) => Err(Error::ty(
                span,
                format!(
                    "missing lifetime: this function type returns a reference, but its \
                     parameters have {n} lifetimes, so it's not clear which it borrows from"
                ),
            )),
        },
        (_, None) => lifetimes.region(l, span),
    }
}

fn resolve_in(
    t: &TypeExpr,
    names: &dyn Fn(&str, Span) -> Option<TypeName>,
    params: &[String],
    lifetimes: &mut Lifetimes,
    mut binder: Option<&mut Binder>,
) -> Result<Type, Error> {
    let go = |t: &TypeExpr, lifetimes: &mut Lifetimes, binder: Option<&mut Binder>| {
        resolve_in(t, names, params, lifetimes, binder)
    };
    match &t.kind {
        TypeExprKind::Named(name, _) if name == "_" => Err(Error::ty(
            t.span,
            "`_` can only stand for a type in type arguments, like `id::<_>`",
        )),
        TypeExprKind::Lifetime(name) => Err(Error::ty(
            t.span,
            format!("expected a type, found lifetime `{name}`"),
        )),
        TypeExprKind::Ref(l, mutable, inner) => {
            let span = Span::new(t.span.start, t.span.start + 1);
            let l = l.as_ref().map(|l| l.name.as_str());
            let region = region(l, span, lifetimes, binder.as_deref_mut())?;
            let inner = go(inner, lifetimes, binder)?;
            Ok(Type::Ref(region, *mutable, Box::new(inner)))
        }
        TypeExprKind::Named(name, args) => {
            // Lifetime arguments come first.
            let lts = args
                .iter()
                .take_while(|a| matches!(a.kind, TypeExprKind::Lifetime(_)))
                .count();
            let (lt_args, args) = args.split_at(lts);
            if let Some(l) = args
                .iter()
                .find(|a| matches!(a.kind, TypeExprKind::Lifetime(_)))
            {
                return Err(Error::ty(
                    l.span,
                    "lifetime arguments must come before type arguments",
                ));
            }
            let (ty, lt_arity, arity) = match names(name, t.span) {
                Some(TypeName::Adt(symbol, l, n)) => {
                    (Type::Adt(symbol, Vec::new(), Vec::new()), l, n)
                }
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
                None if params.contains(name) => (Type::Param(name.clone()), 0, 0),
                None if name == "int" => (Type::Int, 0, 0),
                None if name == "bool" => (Type::Bool, 0, 0),
                None if name == "Box" => (Type::Box(Box::new(Type::Unit)), 0, 1),
                None => return Err(Error::ty(t.span, format!("unknown type `{name}`"))),
            };
            if args.len() != arity {
                let what = match ty {
                    Type::Param(_) => format!("type parameter `{name}`"),
                    _ => format!("`{name}`"),
                };
                return Err(arity_error(&what, arity, args.len(), t.span));
            }
            // Left out, they're all elided, as in `fn f(h: Holder) -> int`.
            let regions = match lt_args.len() {
                0 => (0..lt_arity)
                    .map(|_| region(None, t.span, lifetimes, binder.as_deref_mut()))
                    .collect::<Result<Vec<_>, _>>()?,
                n if n == lt_arity => lt_args
                    .iter()
                    .map(|a| match &a.kind {
                        TypeExprKind::Lifetime(l) => {
                            region(Some(l), a.span, lifetimes, binder.as_deref_mut())
                        }
                        _ => unreachable!("only lifetimes"),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                n => {
                    let message = match lt_arity {
                        0 => format!("`{name}` takes no lifetime arguments"),
                        1 => format!("`{name}` takes 1 lifetime argument but {n} were supplied"),
                        l => format!("`{name}` takes {l} lifetime arguments but {n} were supplied"),
                    };
                    return Err(Error::ty(t.span, message));
                }
            };
            let args = args
                .iter()
                .map(|a| go(a, lifetimes, binder.as_deref_mut()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(match ty {
                Type::Adt(symbol, ..) => Type::Adt(symbol, regions, args),
                Type::Box(_) => Type::Box(Box::new(args.into_iter().next().expect("one"))),
                ty => ty,
            })
        }
        TypeExprKind::Unit => Ok(Type::Unit),
        // A function type of its own, generic over what it leaves out.
        TypeExprKind::Fn(params_, ret) => {
            let mut b = Binder {
                bound: Vec::new(),
                ret: None,
            };
            let params_ = params_
                .iter()
                .map(|p| go(p, lifetimes, Some(&mut b)))
                .collect::<Result<Vec<_>, _>>()?;
            let mut inputs: Vec<&Region> = Vec::new();
            for r in params_.iter().flat_map(Type::regions) {
                if !inputs.contains(&r) {
                    inputs.push(r);
                }
            }
            b.ret = Some(match inputs[..] {
                [r] => Ok(r.clone()),
                _ => Err(inputs.len()),
            });
            let ret = go(ret, lifetimes, Some(&mut b))?;
            Ok(Type::Fn(FnType {
                params: params_,
                ret: Box::new(ret),
                bound: b.bound,
            }))
        }
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
        TypeExprKind::Unit | TypeExprKind::Lifetime(_) => t.kind.clone(),
        TypeExprKind::Fn(params, ret) => TypeExprKind::Fn(
            params.iter().map(|p| name_holes(p, holes)).collect(),
            Box::new(name_holes(ret, holes)),
        ),
        TypeExprKind::Ref(l, m, inner) => {
            TypeExprKind::Ref(l.clone(), *m, Box::new(name_holes(inner, holes)))
        }
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
        if matches!(p.name.as_str(), "int" | "bool" | "Box") {
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

/// Checks the lifetime parameters declared by a function, struct or enum,
/// and returns their names.
pub(super) fn declare_lifetimes(params: &[Ident]) -> Result<Vec<String>, Error> {
    let mut names: Vec<String> = Vec::new();
    for p in params {
        if p.name == "'static" || p.name == "'_" {
            return Err(Error::ty(
                p.span,
                format!("`{}` is a reserved lifetime name", p.name),
            ));
        }
        if names.contains(&p.name) {
            return Err(Error::ty(
                p.span,
                format!("lifetime `{}` is declared twice", p.name),
            ));
        }
        names.push(p.name.clone());
    }
    Ok(names)
}

/// The bounds in `g`, whose lifetime parameters are `lifetimes` and type
/// parameters `params`.
pub(super) fn bounds(
    g: &Generics,
    lifetimes: &[String],
    params: &[String],
) -> Result<Vec<crate::types::Bound>, Error> {
    use crate::types::Bound;
    let mut l = Lifetimes::Body {
        declared: lifetimes,
    };
    let mut out = Vec::new();
    for (subject, after) in &g.bounds {
        for b in after {
            let r = l.region(Some(&b.name), b.span)?;
            if subject.name.starts_with('\'') {
                let s = l.region(Some(&subject.name), subject.span)?;
                out.push(Bound::Region(s, r));
            } else if params.contains(&subject.name) {
                out.push(Bound::Param(subject.name.clone(), r));
            }
        }
    }
    Ok(out)
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
