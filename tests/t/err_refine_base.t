// A refinement of a value that isn't an integer or a `bool` says something
// of it only through its measures.
fn f(o: { v: Option<i64> | v == v }) -> i64 { 1 }

fn main() {}
// error: a refinement can only name `v`, of type `Option<i64>`, by its measures: `v.m()`, where `m` is a `#[measure]`
