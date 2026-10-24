// A written `Discard` could need another trait: here, `Token<T>` would be
// `Discard` only if `T: Copy`. Then proving a type `Discard` could need
// any trait, and through it lead anywhere, even back to itself across
// packages. Derived, a type is `Discard` exactly when its fields are, which
// needs only `Discard` of its parameters.
struct Token<T> { f: fn(T) -> i64 }

impl<T: Copy> Discard for Token<T> {}

fn main() {}
// error: `Discard` can only be derived
