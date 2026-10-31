// What `ensures` says is a `bool`.
fn f(n: usize) ensures n + 1 {}

fn main() {}
// error: expected `bool`, found an integer
