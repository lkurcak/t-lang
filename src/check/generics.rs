//! Type parameters: resolving the names in written types against them, and
//! the rules that keep generic code monomorphisable (see
//! docs/04-generics.md).

use super::scope::source_name;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{FnType, Params, Region, Type};
use std::collections::HashMap;

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
/// it's written in, or `i64` or `bool`.
pub(crate) enum TypeName {
    /// A struct or enum, by its symbol (see `check/names.rs`), with how
    /// many lifetime and type arguments it takes.
    Adt(String, usize, usize),
    /// A type parameter of a function that what it's written in is nested
    /// in, which it can't use. With the keyword and name of what it's
    /// written in: `("fn", "helper")`.
    OuterParam(&'static str, String),
    /// An opaque type, `impl Tr`, in a function's return type: by its
    /// symbol, with the function's lifetime and type parameters, which
    /// are its arguments there.
    Opaque(String, Vec<String>, Params),
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
    /// In a closure's parameters, as for a function type's: each lifetime
    /// left out is one the closure is generic over, named `'^0`, `'^1` and
    /// so on, added to `bound`, which each call picks.
    ClosureParams {
        declared: &'a [String],
        bound: Vec<String>,
    },
    /// In the fields of a struct or enum, where none can be left out.
    Fields { declared: &'a [String] },
    /// In what an impl says an associated type is, or an equality bound
    /// does, where none can be left out either.
    Assoc { declared: &'a [String] },
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
            | Lifetimes::ClosureParams { declared, .. }
            | Lifetimes::Fields { declared }
            | Lifetimes::Assoc { declared } => *declared,
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
                     does, as in `fn f<'a>(x: &'a i64, y: &i64) -> &'a i64`"
                ),
            )),
            Lifetimes::ClosureParams { bound, .. } => {
                let name = format!("'^{}", bound.len());
                bound.push(name.clone());
                Ok(Region::Named(name))
            }
            Lifetimes::Body { .. } | Lifetimes::Any => Ok(Region::Erased),
            Lifetimes::Fields { .. } => Err(Error::ty(
                span,
                "missing lifetime: a reference in a struct or enum needs one, as in \
                 `struct S<'a> { r: &'a i64 }`",
            )),
            Lifetimes::Assoc { .. } => Err(Error::ty(
                span,
                "missing lifetime: a reference here needs one, a lifetime of the type the impl \
                 is for, as in `impl<'a> Tr for &'a i64 { type Out = &'a i64; }`, or \
                 `'static`",
            )),
        }
    }
}

/// What a projection `<T as Tr>::Item` or `T::Item`, written as the type
/// given last, is, given what's before the `as` (`T`), the trait if it's
/// written, and the associated type's name.
pub(crate) type ProjNames<'a> =
    dyn Fn(Type, Option<&Ident>, &Ident, &TypeExpr, Kind) -> Result<Type, Error> + 'a;

/// What a name written must be: a type, or effects (`with e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Type,
    Effects,
}

/// That slice `t` is written somewhere other than right after `&` or
/// `&mut`, where it would be a value of its own.
pub(crate) fn unsized_slice(span: Span, t: impl std::fmt::Display) -> Error {
    match t.to_string().as_str() {
        "str" => Error::ty(
            span,
            "`str` can only be behind a shared reference, as in `&str`",
        ),
        t => Error::ty(
            span,
            format!("a slice `{t}` can only be behind a reference, as in `&{t}`"),
        ),
    }
}

/// Is `t` the built-in `str`, written as `name`, which `names` says no
/// declared type is?
fn is_str(t: &TypeExpr, names: &dyn Fn(&str, &TypeExpr) -> Option<TypeName>) -> bool {
    matches!(&t.kind, TypeExprKind::Named(n, args) if n == "str" && args.is_empty())
        && names("str", t).is_none()
}

/// Turns a type as written (`TypeExpr`) into a type as understood (`Type`).
/// `names` says what the name of a type written refers to, and `params` are the type
/// parameters in scope, for names it says nothing about. `proj` says what a
/// projection is. `lifetimes` says what lifetimes mean.
pub(crate) fn resolve(
    t: &TypeExpr,
    names: &dyn Fn(&str, &TypeExpr) -> Option<TypeName>,
    proj: &ProjNames,
    params: &Params,
    lifetimes: &mut Lifetimes,
) -> Result<Type, Error> {
    resolve_in(t, names, proj, params, lifetimes, None)
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
    names: &dyn Fn(&str, &TypeExpr) -> Option<TypeName>,
    proj: &ProjNames,
    params: &Params,
    lifetimes: &mut Lifetimes,
    mut binder: Option<&mut Binder>,
) -> Result<Type, Error> {
    let go = |t: &TypeExpr, lifetimes: &mut Lifetimes, binder: Option<&mut Binder>| {
        resolve_in(t, names, proj, params, lifetimes, binder)
    };
    match &t.kind {
        TypeExprKind::Proj(inner, tr, name) => {
            let inner = go(inner, lifetimes, binder)?;
            proj(inner, tr.as_ref(), name, t, Kind::Type)
        }
        // The places a refinement can be take it off first (see
        // `check/refine.rs`).
        TypeExprKind::Refined(..) => Err(Error::ty(
            t.span,
            "a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field",
        )),
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
            // The one place a slice can be, and, if it's shared, a `str`.
            let inner = match &inner.kind {
                TypeExprKind::Slice(elem) => Type::Slice(Box::new(go(elem, lifetimes, binder)?)),
                _ if is_str(inner, names) && !*mutable => Type::Str,
                _ => go(inner, lifetimes, binder)?,
            };
            Ok(Type::Ref(region, *mutable, Box::new(inner)))
        }
        TypeExprKind::Slice(_) => Err(unsized_slice(t.span, t)),
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
            let (ty, lt_arity, arity) = match names(name, t) {
                // The alloc library's `Box`, which the checker knows.
                Some(TypeName::Adt(symbol, ..)) if symbol == super::BOX => {
                    (Type::Box(Box::new(Type::Unit)), 0, 1)
                }
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
                Some(TypeName::Opaque(..)) => unreachable!("an opaque type is written `impl`"),
                None if params.is_effect(name) => {
                    return Err(Error::ty(
                        t.span,
                        format!("`{name}` is an effect parameter, not a type"),
                    ));
                }
                None if params.contains(name) => (Type::Param(name.clone()), 0, 0),
                None if let Some(t) = crate::types::IntTy::named(name) => (Type::Int(t), 0, 0),
                None if name == "bool" => (Type::Bool, 0, 0),
                None if name == "char" => (Type::Char, 0, 0),
                None if name == "str" => return Err(unsized_slice(t.span, "str")),
                None => return Err(Error::ty(t.span, format!("unknown type `{name}`"))),
            };
            if args.len() != arity {
                let what = match ty {
                    Type::Param(_) => format!("type parameter `{name}`"),
                    _ => format!("`{name}`"),
                };
                return Err(arity_error(&what, arity, args.len(), t.span));
            }
            // Left out, they're all elided, as in `fn f(h: Holder) -> i64`.
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
        TypeExprKind::Never => Ok(Type::Never),
        // Where it can be, its symbol is known by its node.
        TypeExprKind::Impl(_) => match names("impl", t) {
            Some(TypeName::Opaque(symbol, lifetimes, params)) => Ok(Type::Opaque(
                symbol,
                lifetimes.into_iter().map(Region::Named).collect(),
                params.iter().cloned().map(Type::Param).collect(),
            )),
            _ => Err(Error::ty(
                t.span,
                "`impl` types can only be written in a function's return type",
            )),
        },
        // A function type of its own, generic over what it leaves out.
        TypeExprKind::Fn(params_, ret, effects) => {
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
            let effects = match effects {
                Some(e) => resolve_effects(e, names, proj, params, lifetimes)?,
                None => Type::no_effects(),
            };
            Ok(Type::Fn(FnType {
                params: params_,
                ret: Box::new(ret),
                bound: b.bound,
                effects: Box::new(effects),
            }))
        }
    }
}

/// Turns effects as written (`with {alloc, e}`) into the set they are, as
/// `resolve` does a type: `params` are the type and effect parameters in
/// scope.
pub(crate) fn resolve_effects(
    e: &EffectSet,
    names: &dyn Fn(&str, &TypeExpr) -> Option<TypeName>,
    proj: &ProjNames,
    params: &Params,
    lifetimes: &mut Lifetimes,
) -> Result<Type, Error> {
    let mut items = Vec::new();
    for item in &e.items {
        items.push(match &item.kind {
            EffectKind::Named(n) => match n.name.as_str() {
                "panic" => Type::Panic,
                "alloc" => Type::Alloc,
                "unsafe" => Type::Unsafe,
                name if params.is_effect(name) => Type::Param(name.to_string()),
                name if params.contains(&name.to_string()) => {
                    return Err(Error::ty(
                        n.span,
                        format!(
                            "`{name}` is a type parameter, not an effect; declare it as \
                             `effect {name}`"
                        ),
                    ));
                }
                name => {
                    return Err(Error::ty(n.span, format!("there's no effect `{name}`")));
                }
            },
            // As the projection it is, which is how `proj` knows it.
            EffectKind::Proj(base, tr, name) => {
                let written = TypeExpr {
                    kind: TypeExprKind::Proj(base.clone(), tr.clone(), name.clone()),
                    span: item.span,
                    id: item.id,
                };
                let base = resolve_in(base, names, proj, params, lifetimes, None)?;
                proj(base, tr.as_ref(), name, &written, Kind::Effects)?
            }
        });
    }
    Ok(Type::effects(items))
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
        TypeExprKind::Unit | TypeExprKind::Never | TypeExprKind::Lifetime(_) => t.kind.clone(),
        TypeExprKind::Fn(params, ret, effects) => TypeExprKind::Fn(
            params.iter().map(|p| name_holes(p, holes)).collect(),
            Box::new(name_holes(ret, holes)),
            effects.clone(),
        ),
        TypeExprKind::Ref(l, m, inner) => {
            TypeExprKind::Ref(l.clone(), *m, Box::new(name_holes(inner, holes)))
        }
        TypeExprKind::Slice(inner) => TypeExprKind::Slice(Box::new(name_holes(inner, holes))),
        TypeExprKind::Proj(inner, tr, name) => {
            TypeExprKind::Proj(Box::new(name_holes(inner, holes)), tr.clone(), name.clone())
        }
        // Rejected wherever there can be holes.
        TypeExprKind::Impl(_) | TypeExprKind::Refined(..) => t.kind.clone(),
    };
    TypeExpr {
        kind,
        span: t.span,
        id: t.id,
    }
}

/// Checks the type and effect parameters declared by a function, impl,
/// struct or enum, and returns them.
pub(super) fn declare(g: &Generics) -> Result<Params, Error> {
    let mut names: Vec<String> = Vec::new();
    for p in &g.types {
        if g.is_effect(p) && ["panic", "alloc"].contains(&p.name.as_str()) {
            return Err(Error::ty(
                p.span,
                format!(
                    "`{}` is a built-in effect, so it can't be an effect parameter's name",
                    p.name
                ),
            ));
        }
        if p.name == "_" {
            return Err(Error::ty(
                p.span,
                "a type parameter needs a name; `_` means \"infer this\" in type arguments",
            ));
        }
        if crate::types::is_builtin_type(&p.name) {
            return Err(Error::ty(
                p.span,
                format!("`{}` is a built-in type", p.name),
            ));
        }
        if p.name == "Self" {
            return Err(Error::ty(
                p.span,
                "`Self` means the type of the impl or trait it's in, so it can't be a type \
                 parameter's name",
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
    Ok(Params::new(names, g.effect_names()))
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

/// The lifetime bounds in `g`, whose lifetime parameters are `lifetimes`
/// and type parameters `params`.
pub(super) fn bounds(
    g: &Generics,
    lifetimes: &[String],
    params: &Params,
) -> Result<Vec<crate::types::Bound>, Error> {
    use crate::types::Bound;
    let mut l = Lifetimes::Body {
        declared: lifetimes,
    };
    let mut out = Vec::new();
    for (subject, after) in &g.bounds {
        // The traits are for `Checker::preds`.
        for b in after
            .iter()
            .map(|b| &b.name)
            .filter(|b| b.name.starts_with('\''))
        {
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

/// How deep type parameter `p` is in `t`, at its deepest, if it's there:
/// 0 for `p` itself, 1 in `Option<p>`. `UNBOUNDED` if it's in a projection:
/// `T::Out` can be a bigger type than `T` by any amount (`W<W<T>>`, say),
/// which only the impls say, and what a copy of a generic function needs
/// mustn't depend on which impls there are.
fn depth_of(t: &Type, p: &str) -> Option<i64> {
    match t {
        Type::Param(q) if q == p => Some(0),
        Type::Proj(s, ..) => depth_of(s, p).map(|_| UNBOUNDED),
        t => t
            .parts()
            .filter_map(|t| depth_of(t, p))
            .max()
            .map(|d| if d == UNBOUNDED { d } else { d + 1 }),
    }
}

/// The weight of an edge that can make types deeper by any amount; see
/// `polymorphic_recursion`.
const UNBOUNDED: i64 = i64::MAX / 4;

/// One generic thing using another: function `from` calls or names
/// function `to`, or struct or enum `from` has a field that mentions `to`,
/// with type arguments `args`, which may mention `from`'s own parameters.
/// Functions are named by their symbols.
pub(super) struct Use<'a> {
    pub(super) from: &'a str,
    pub(super) to: &'a str,
    pub(super) args: Vec<Type>,
    /// `(j, i, depth)`: `to`'s `j`-th type parameter is a part of what
    /// `from`'s `i`-th stands for, `depth` type constructors inside it,
    /// rather than an argument built from those. That's how a call of a
    /// trait's method at `Self = T` gets to an impl's method: for `impl<U>
    /// Show for Option<U>`, `U` is inside `T`, one down. `usize::MAX` if
    /// it's inside what a projection on it stands for, which can be bigger
    /// than it.
    pub(super) inside: Vec<(usize, usize, usize)>,
    pub(super) span: Span,
}

/// Rejects polymorphic recursion that would make monomorphisation go on
/// forever, like `fn f<T>(x: T) { f(Option::Some(x)) }`: `f<i64>` needs
/// `f<Option<i64>>`, which needs `f<Option<Option<i64>>>`, and so on.
///
/// Every copy monomorphisation makes is of some item with arguments built
/// from the arguments of the copy that needed it, or found inside them.
/// That's a graph problem: there's a node for each parameter of each item,
/// and a use of `to<.., A, ..>` in `from` gives an edge from each parameter
/// `p` of `from` that `A` mentions to `to`'s parameter, *weighed* by how
/// deep in `A` `p` is: how many type constructors it adds. A parameter
/// found inside another, as a call of a trait's method finds an impl's,
/// gives an edge of negative weight: it takes constructors away. Along a
/// path, the weights bound how much deeper the types get. So
/// monomorphisation finishes exactly when no cycle weighs more than 0:
/// then the types stay as deep as some bound, and there are only so many
/// of those. (This extends how .NET's ECMA-335 spec rules out "expansive"
/// generic types, where an edge only either grows or doesn't.)
///
/// `params` holds each item's type parameters. `current` says whether a
/// use is in the package being checked: those before it kept this on their
/// own, so a cycle that weighs more than 0 now has a use of its on it,
/// which is what's reported.
pub(super) fn polymorphic_recursion(
    uses: &[Use],
    params: &HashMap<&str, &[String]>,
    current: &dyn Fn(Span) -> bool,
) -> Result<(), Error> {
    // Edges between `(item, parameter index)` nodes, with their weights.
    type Node<'a> = (&'a str, usize);
    let mut edges: Vec<(Node, Node, i64, &Use)> = Vec::new();
    for u in uses {
        let Some(from_params) = params.get(u.from) else {
            continue;
        };
        for (j, arg) in u.args.iter().enumerate() {
            for (i, p) in from_params.iter().enumerate() {
                if let Some(d) = depth_of(arg, p) {
                    edges.push(((u.from, i), (u.to, j), d, u));
                }
            }
        }
        for &(j, i, d) in &u.inside {
            let w = match d {
                usize::MAX => UNBOUNDED,
                d => -(d as i64),
            };
            edges.push(((u.from, i), (u.to, j), w, u));
        }
    }
    // An edge through a projection outweighs every edge that takes
    // constructors away put together, so any cycle through it weighs more
    // than 0: it can make types deeper by more than all of them can make
    // them shallower.
    let unbounded: i64 = 1 + edges.iter().map(|e| e.2.min(0).abs()).sum::<i64>();
    for e in &mut edges {
        if e.2 == UNBOUNDED {
            e.2 = unbounded;
        }
    }
    let mut nodes: Vec<Node> = edges.iter().flat_map(|e| [e.0, e.1]).collect();
    nodes.sort();
    nodes.dedup();
    let index = |n: &Node| nodes.binary_search(n).expect("a node");
    // The heaviest path from `start` to each node, or `None` if there's
    // none, by Bellman-Ford. `i64::MAX` if it can be as heavy as you like,
    // through a cycle that weighs more than 0.
    let heaviest = |start: Option<Node>| -> Vec<Option<i64>> {
        let mut dist: Vec<Option<i64>> = match start {
            Some(s) => (0..nodes.len())
                .map(|i| (i == index(&s)).then_some(0))
                .collect(),
            None => vec![Some(0); nodes.len()],
        };
        // After as many rounds as there are nodes, a path that still gets
        // heavier goes round such a cycle, and so does what it leads to.
        for round in 0..2 * nodes.len() + 1 {
            let mut changed = false;
            for &(a, b, w, _) in &edges {
                let (a, b) = (index(&a), index(&b));
                let Some(da) = dist[a] else {
                    continue;
                };
                let new = match round >= nodes.len() || da == i64::MAX {
                    true => i64::MAX,
                    false => da + w,
                };
                if dist[b].is_none_or(|db| new > db) {
                    dist[b] = Some(new);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        dist
    };
    if !heaviest(None).contains(&Some(i64::MAX)) {
        return Ok(());
    }
    // An edge on a cycle that does, to point at: of the package being
    // checked, in the order of `uses`, and one that makes types deeper if
    // there's one.
    let on_cycle = |&&(a, b, w, _): &&(Node, Node, i64, &Use)| {
        let back = heaviest(Some(b))[index(&a)];
        back.is_some_and(|back| back == i64::MAX || back + w > 0)
    };
    let own = |e: &&(Node, Node, i64, &Use)| current(e.3.span);
    let found = (edges.iter().filter(own).filter(|e| e.2 > 0).find(on_cycle))
        .or_else(|| edges.iter().filter(own).find(on_cycle))
        .or_else(|| edges.iter().filter(|e| e.2 > 0).find(on_cycle));
    let Some(&(_, b, w, u)) = found else {
        unreachable!("a cycle that weighs more than 0 has an edge that does")
    };
    let arg = &u.args[b.1];
    let to_p = &params[u.to][b.1];
    let (from, to) = (source_name(u.from), source_name(u.to));
    let message = if w <= 0 {
        format!(
            "polymorphic recursion: this use of `{to}` leads back to `{from}` through uses \
             with a bigger type each time round, so monomorphising them would never end"
        )
    } else if u.from == u.to {
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
    Err(Error::ty(u.span, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library's `g<T>` uses `h<Option<T>>`, and alone that's fine; a
    /// program's `h<U>` uses `g<U>`, which makes a cycle that gets bigger
    /// each time round, though only the library's use adds to it. The
    /// library checked on its own, so the program's use is the one at
    /// fault, and the error is there.
    #[test]
    fn a_cycle_through_a_library_is_the_programs_fault() {
        let t = |p: &str| Type::Param(p.into());
        let option = |a: Type| Type::Adt("core::option::Option".into(), Vec::new(), vec![a]);
        let library = Use {
            from: "lib::g",
            to: "h",
            args: vec![option(t("T"))],
            inside: Vec::new(),
            span: Span::new(10, 11),
        };
        let program = Use {
            from: "h",
            to: "lib::g",
            args: vec![t("U")],
            inside: Vec::new(),
            span: Span::new(100, 101),
        };
        let (gt, hu) = (["T".to_string()], ["U".to_string()]);
        let params = HashMap::from([("lib::g", &gt[..]), ("h", &hu[..])]);
        let current = |s: Span| s.start >= 50;
        let alone = polymorphic_recursion(std::slice::from_ref(&library), &params, &current);
        assert!(alone.is_ok());
        let e = polymorphic_recursion(&[library, program], &params, &current).unwrap_err();
        assert_eq!(e.span, Span::new(100, 101), "{}", e.message);
        assert!(e.message.contains("leads back to `h`"), "{}", e.message);
    }
}
