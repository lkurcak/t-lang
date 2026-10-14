// Two type parameters are two different types, even if a caller picks the
// same type for both.
fn first<A, B>(a: A, b: B) -> A { b }
fn main() { print(first(1, 2)); }
// error: expected `A`, found `B`
