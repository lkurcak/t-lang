// Inside `as_i64`, `T` could be any type, so it isn't an `i64`.
fn as_i64<T>(x: T) -> i64 { x }
fn main() { print(as_i64(1i64)); }
// error: expected `i64`, found `T`
