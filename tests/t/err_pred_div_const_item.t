// A const item isn't a constant of literals: its value is the checker's
// only once its body is checked.
const PAGE: u64 = 4096;

fn f(a: { v: u64 | v % PAGE == 0 }) {}

fn main() {}
// error: a refinement can use `%` only by a constant: `PAGE` isn't one
