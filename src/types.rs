//! Types as the checker understands them.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    /// The type with exactly one value, `()`. "Nothing interesting to return."
    Unit,
    /// `fn(A, B) -> R`. The arrow type `A × B → R` of the lambda calculus.
    Fn(Vec<Type>, Box<Type>),
    /// A struct or enum, by name. Its definition lives in an `Adts` table.
    ///
    /// Types are compared by *name* (nominal typing): `struct A { x: int }`
    /// and `struct B { x: int }` are different types even though they look
    /// the same. The alternative, structural typing, would compare what's
    /// inside instead.
    Adt(String),
    /// A type not known yet: a *type variable*, standing for some type the
    /// checker will work out from how the value is used (see docs/03-inference.md).
    /// The number indexes the checker's table of what each one has turned out
    /// to be. Variables only ever exist while a function body is checked.
    Var(u32),
}

/// What a struct or enum is made of.
#[derive(Debug, Clone)]
pub enum AdtDef {
    /// A *product* type: a value has every field.
    Struct(Vec<(String, Type)>),
    /// A *sum* type: a value is exactly one of the variants, and also holds
    /// that variant's fields.
    Enum(Vec<(String, Vec<Type>)>),
}

/// Every struct and enum in the program, by name.
pub type Adts = HashMap<String, AdtDef>;

// Looking inside struct and enum values. Everything that needs a field's type
// goes through these instead of reading an `AdtDef`, so that once types take
// arguments (generics, step 4) they're substituted in one place.

/// The fields of `ty`, if it's a struct.
pub fn struct_fields(adts: &Adts, ty: &Type) -> Option<Vec<(String, Type)>> {
    match ty {
        Type::Adt(name) => match &adts[name] {
            AdtDef::Struct(fields) => Some(fields.clone()),
            AdtDef::Enum(_) => None,
        },
        _ => None,
    }
}

/// The fields of variant `i` of `ty`, which must be an enum.
pub fn variant_fields(adts: &Adts, ty: &Type, i: usize) -> Vec<Type> {
    match ty {
        Type::Adt(name) => match &adts[name] {
            AdtDef::Enum(variants) => variants[i].1.clone(),
            AdtDef::Struct(_) => unreachable!("variant of struct `{name}`"),
        },
        _ => unreachable!("variant of `{ty}`"),
    }
}

/// The types of every field of every variant of `ty`. None, unless it's a
/// struct or enum.
pub fn all_fields(adts: &Adts, ty: &Type) -> Vec<Type> {
    match ty {
        Type::Adt(name) => match &adts[name] {
            AdtDef::Struct(fields) => fields.iter().map(|(_, t)| t.clone()).collect(),
            AdtDef::Enum(variants) => variants.iter().flat_map(|(_, ts)| ts.clone()).collect(),
        },
        _ => Vec::new(),
    }
}

impl Type {
    /// The types directly inside this one: a function type's parameters and
    /// result. Code that walks a type goes through this and `map`, so a new
    /// kind of type with parts only has to be taught to these two.
    pub fn parts(&self) -> impl Iterator<Item = &Type> {
        let (params, ret): (&[Type], Option<&Type>) = match self {
            Type::Fn(params, ret) => (params, Some(ret)),
            Type::Int | Type::Bool | Type::Unit | Type::Adt(_) | Type::Var(_) => (&[], None),
        };
        params.iter().chain(ret)
    }

    /// This type, with each of its parts replaced by `f(part)`.
    pub fn map(&self, mut f: impl FnMut(&Type) -> Type) -> Type {
        match self {
            Type::Fn(params, ret) => {
                Type::Fn(params.iter().map(&mut f).collect(), Box::new(f(ret)))
            }
            Type::Int | Type::Bool | Type::Unit | Type::Adt(_) | Type::Var(_) => self.clone(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => f.write_str("int"),
            Type::Bool => f.write_str("bool"),
            Type::Unit => f.write_str("()"),
            Type::Fn(params, ret) => {
                f.write_str("fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::Adt(name) => f.write_str(name),
            // Like Rust: "some type, not known yet".
            Type::Var(_) => f.write_str("_"),
        }
    }
}
