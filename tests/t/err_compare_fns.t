fn f(x: i64) -> i64 { x }
fn main() { let _x = f == f; }
// error: cannot compare functions with `==`
