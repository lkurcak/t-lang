//! Types as the checker understands them.

use crate::check::source_name;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
    /// The type with exactly one value, `()`. "Nothing interesting to return."
    Unit,
    /// `fn(A, B) -> R`. The arrow type `A × B → R` of the lambda calculus.
    Fn(FnType),
    /// A struct or enum, by its symbol, with its type arguments: `Shape`,
    /// or `Option<int>`. Its definition lives in an `Adts` table. The
    /// symbol is its name, unless it's a local type (see `check/names.rs`).
    /// `Display` shows the name, as written.
    ///
    /// Types are compared by *name* (nominal typing): `struct A { x: int }`
    /// and `struct B { x: int }` are different types even though they look
    /// the same. The alternative, structural typing, would compare what's
    /// inside instead.
    Adt(String, Vec<Type>),
    /// A type parameter, like the `T` in `fn id<T>(x: T) -> T`, inside the
    /// function or type that declares it. There it stands for a type the
    /// caller picks, so nothing is known about it: it's equal only to itself
    /// (see docs/04-generics.md).
    Param(String),
    /// A type not known yet: a *type variable*, standing for some type the
    /// checker will work out from how the value is used (see docs/03-inference.md).
    /// The number indexes the checker's table of what each one has turned out
    /// to be. Variables only ever exist while a function body is checked.
    Var(u32),
}

/// A function type's parts. A struct rather than fields of `Type::Fn`, so
/// that a function type can grow more parts without touching everything
/// that only looks at these: step 7 adds the effects a call may have, like
/// panicking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnType {
    pub params: Vec<Type>,
    pub ret: Box<Type>,
}

impl Type {
    /// `fn(params) -> ret`.
    pub fn func(params: Vec<Type>, ret: Type) -> Type {
        Type::Fn(FnType {
            params,
            ret: Box::new(ret),
        })
    }
}

/// What a struct or enum is made of.
#[derive(Debug, Clone)]
pub struct AdtDef {
    /// Its type parameters: `["T"]` for `enum Option<T>`. Field types use
    /// them as `Type::Param`.
    pub params: Vec<String>,
    pub kind: AdtKind,
}

#[derive(Debug, Clone)]
pub enum AdtKind {
    /// A *product* type: a value has every field.
    Struct(Vec<(String, Type)>),
    /// A *sum* type: a value is exactly one of the variants, and also holds
    /// that variant's fields.
    Enum(Vec<(String, Vec<Type>)>),
}

/// Every struct and enum in the program, by name.
pub type Adts = HashMap<String, AdtDef>;

// Looking inside struct and enum values. Everything that needs a field's type
// goes through these instead of reading an `AdtDef`, because the types an
// `AdtDef` holds mention its type parameters, and these put the type's
// arguments in their place: the field of `Option<int>::Some` is an `int`.

/// What `name<args>` is made of, with `args` substituted for the parameters.
fn instantiate(adts: &Adts, name: &str, args: &[Type]) -> AdtKind {
    let def = &adts[name];
    let s = Subst::new(&def.params, args);
    match &def.kind {
        AdtKind::Struct(fields) => AdtKind::Struct(
            fields
                .iter()
                .map(|(f, t)| (f.clone(), s.apply(t)))
                .collect(),
        ),
        AdtKind::Enum(variants) => AdtKind::Enum(
            variants
                .iter()
                .map(|(v, ts)| (v.clone(), ts.iter().map(|t| s.apply(t)).collect()))
                .collect(),
        ),
    }
}

/// The fields of `ty`, if it's a struct.
pub fn struct_fields(adts: &Adts, ty: &Type) -> Option<Vec<(String, Type)>> {
    match ty {
        Type::Adt(name, args) => match instantiate(adts, name, args) {
            AdtKind::Struct(fields) => Some(fields),
            AdtKind::Enum(_) => None,
        },
        _ => None,
    }
}

/// The fields of variant `i` of `ty`, which must be an enum.
pub fn variant_fields(adts: &Adts, ty: &Type, i: usize) -> Vec<Type> {
    match ty {
        Type::Adt(name, args) => match instantiate(adts, name, args) {
            AdtKind::Enum(mut variants) => variants.swap_remove(i).1,
            AdtKind::Struct(_) => unreachable!("variant of struct `{name}`"),
        },
        _ => unreachable!("variant of `{ty}`"),
    }
}

/// The types of every field of every variant of `ty`. None, unless it's a
/// struct or enum.
pub fn all_fields(adts: &Adts, ty: &Type) -> Vec<Type> {
    match ty {
        Type::Adt(name, args) => match instantiate(adts, name, args) {
            AdtKind::Struct(fields) => fields.into_iter().map(|(_, t)| t).collect(),
            AdtKind::Enum(variants) => variants.into_iter().flat_map(|(_, ts)| ts).collect(),
        },
        _ => Vec::new(),
    }
}

/// A *substitution*: which type each type parameter stands for. Applying
/// one to `fn(T) -> Option<T>` with `T := int` gives `fn(int) -> Option<int>`.
#[derive(Debug, Clone, Default)]
pub struct Subst(HashMap<String, Type>);

impl Subst {
    /// Parameter `params[i]` is `args[i]`.
    pub fn new(params: &[String], args: &[Type]) -> Subst {
        assert_eq!(params.len(), args.len(), "wrong number of type arguments");
        Subst(params.iter().cloned().zip(args.iter().cloned()).collect())
    }

    /// `t`, with every parameter this substitution knows replaced.
    pub fn apply(&self, t: &Type) -> Type {
        match t {
            Type::Param(p) => self.0.get(p).cloned().unwrap_or_else(|| t.clone()),
            t => t.map(|p| self.apply(p)),
        }
    }
}

impl Type {
    /// The types directly inside this one: a function type's parameters and
    /// result, and a struct or enum's type arguments. Code that walks a type
    /// goes through this and `map`, so a new kind of type with parts only
    /// has to be taught to these two (and `same_shape`).
    pub fn parts(&self) -> impl Iterator<Item = &Type> {
        let (params, ret): (&[Type], Option<&Type>) = match self {
            Type::Fn(f) => (&f.params, Some(&f.ret)),
            Type::Adt(_, args) => (args, None),
            Type::Int | Type::Bool | Type::Unit | Type::Param(_) | Type::Var(_) => (&[], None),
        };
        params.iter().chain(ret)
    }

    /// This type, with each of its parts replaced by `f(part)`.
    pub fn map(&self, mut f: impl FnMut(&Type) -> Type) -> Type {
        match self {
            Type::Fn(sig) => Type::func(sig.params.iter().map(&mut f).collect(), f(&sig.ret)),
            Type::Adt(name, args) => Type::Adt(name.clone(), args.iter().map(f).collect()),
            Type::Int | Type::Bool | Type::Unit | Type::Param(_) | Type::Var(_) => self.clone(),
        }
    }

    /// Do `self` and `other` have the same outermost shape, so that they're
    /// equal exactly when their `parts` are? `fn(A) -> B` and `fn(C) -> D`
    /// do, `Option<A>` and `Option<B>` do, `int` and `bool` don't.
    pub fn same_shape(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Fn(f), Type::Fn(g)) => f.params.len() == g.params.len(),
            (Type::Adt(a, xs), Type::Adt(b, ys)) => a == b && xs.len() == ys.len(),
            (Type::Var(_), _) | (_, Type::Var(_)) => false,
            (a, b) => a == b,
        }
    }
}

impl Type {
    /// Like `to_string`, but with the symbols of structs and enums rather
    /// than their names, so that two types of the same name show
    /// differently.
    pub fn symbols(&self) -> String {
        match self {
            Type::Fn(sig) => {
                let params: Vec<String> = sig.params.iter().map(Type::symbols).collect();
                format!("fn({}) -> {}", params.join(", "), sig.ret.symbols())
            }
            Type::Adt(name, args) if args.is_empty() => name.clone(),
            Type::Adt(name, args) => {
                let args: Vec<String> = args.iter().map(Type::symbols).collect();
                format!("{name}<{}>", args.join(", "))
            }
            t => t.to_string(),
        }
    }
}

/// `a, b, c`
pub fn comma_list<T: fmt::Display>(items: &[T]) -> String {
    let items: Vec<String> = items.iter().map(T::to_string).collect();
    items.join(", ")
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => f.write_str("int"),
            Type::Bool => f.write_str("bool"),
            Type::Unit => f.write_str("()"),
            Type::Fn(sig) => write!(f, "fn({}) -> {}", comma_list(&sig.params), sig.ret),
            Type::Adt(name, args) if args.is_empty() => f.write_str(&source_name(name)),
            Type::Adt(name, args) => write!(f, "{}<{}>", source_name(name), comma_list(args)),
            Type::Param(name) => f.write_str(name),
            // Like Rust: "some type, not known yet".
            Type::Var(_) => f.write_str("_"),
        }
    }
}

/// The type of a function, which may be generic: `∀T. fn(T) -> T`
/// is `Scheme { params: ["T"], ty: fn(T) -> T }`. Each use of the function
/// *instantiates* it, picking a type for each parameter.
///
/// Only functions have schemes, and they're the only place a `∀` can be:
/// at the very front, around a whole function type. Everything else has a
/// plain `Type`, with no `∀` inside. That's *rank-1* (or *prenex*)
/// polymorphism (see docs/04-generics.md).
#[derive(Debug, Clone)]
pub struct Scheme {
    pub params: Vec<String>,
    pub ty: Type,
}
