// A predicate can't divide by zero: it's total.
fn f(a: { v: i64 | v / 0 == 1 }) {}

fn main() {}
// error: a refinement can't divide by zero
