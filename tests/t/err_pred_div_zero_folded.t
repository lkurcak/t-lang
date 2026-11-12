// A divisor is a constant of literals, which is folded: `2 - 2` is zero.
fn f(a: { v: i64 | v % (2 - 2) == 1 }) {}

fn main() {}
// error: a refinement can't divide by zero
