// A `discard` of its own could need another trait: here, `Token<T>` would
// be `Discard` only if `T: Copy`. Then proving a type `Discard` could need
// any trait, and through it lead anywhere, even back to itself across
// packages. Its impl may only need `Discard` of the type's parameters, as
// a derived one does.
struct Token<T> { f: fn(T) -> i64 }

impl<T: Copy> Discard for Token<T> {
    fn discard(self) {}
}

fn main() {}
// error: an impl of `Discard` may only need its type parameters to be `Discard`, not `Copy`
