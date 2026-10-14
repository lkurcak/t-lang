// Inside `add_one`, `T` could be any type, so it can't be added to.
fn add_one<T>(x: T) -> int { x + 1 }
fn main() { print(add_one(1)); }
// error: expected `int`, found `T`
