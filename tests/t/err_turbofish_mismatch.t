// Written type arguments are checked like inferred ones.
fn id<T>(x: T) -> T { x }
fn main() { print(id::<bool>(1)); }
// error: expected `bool`, found `int`
