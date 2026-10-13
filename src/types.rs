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
        }
    }
}
