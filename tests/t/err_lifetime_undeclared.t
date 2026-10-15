fn f(x: &'a int) -> int { *x }

fn main() {}
// error: use of undeclared lifetime `'a`
