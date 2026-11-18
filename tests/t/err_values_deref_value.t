// Only a reference or a box can be dereferenced
// (docs/85-values-in-predicates.md).
fn f(n: u8) -> { r: u8 | r == *n } { n }

fn main() {}
// error: `n` is a `u8`, which isn't a reference or a box, so it can't be dereferenced
