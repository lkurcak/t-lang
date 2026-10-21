// `n` can change, so which of its values would the refinement mean?
fn f(mut n: int, k: { v: int | v < n }) -> int { k }

fn main() {}
// error: a refinement can't name `n`, which is `mut`
