//! Types as the checker understands them.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    /// The type with exactly one value, `()`. "Nothing interesting to return."
    Unit,
    /// `fn(A, B) -> R`. The arrow type `A × B → R` of the lambda calculus.
    Fn(Vec<Type>, Box<Type>),
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
        }
    }
}
