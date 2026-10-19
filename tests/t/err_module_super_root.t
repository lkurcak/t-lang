fn f() -> int { 1 }

fn main() { print(super::f()); }
// error: `super` at the crate root
