// `n` can change, so which of its values would the refinement mean?
fn f(mut n: i64, k: { v: i64 | v < n }) -> i64 { k }

fn main() {}
// error: a refinement can't name `n`, which is `mut`
