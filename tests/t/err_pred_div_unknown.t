// Only by a constant: `v % n` isn't linear.
fn f(n: i64, a: { v: i64 | v % n == 1 }) {}

fn main() {}
// error: a refinement can use `%` only by a constant: `n` isn't one
