// A refinement names a value that isn't an integer or a `bool` only by
// what its measures say of it.
fn f(s: &[i64], t: { t: &[i64] | t == s }) {}

fn main() {}
// error: a refinement can only name `t`, of type `&[i64]`, by its measures: `t.m()`, where `m` is a `#[measure]`
