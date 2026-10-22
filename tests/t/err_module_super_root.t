fn f() -> i64 { 1 }

fn main() { print(super::f()); }
// error: `super` at the crate root
