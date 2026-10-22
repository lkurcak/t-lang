fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }
fn is_zero(x: i64) -> bool { x == 0 }
fn main() -> i64 { apply(is_zero, 1) }
// error: expected `fn(i64) -> i64`, found `fn(i64) -> bool`
