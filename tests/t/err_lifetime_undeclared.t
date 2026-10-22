fn f(x: &'a i64) -> i64 { *x }

fn main() {}
// error: use of undeclared lifetime `'a`
