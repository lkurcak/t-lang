fn f(x: i64) -> i64 { x }
fn main() -> bool { f == f }
// error: cannot compare functions with `==`
