// A parameter's refinement can only name the parameters before it.
fn f(k: { v: i64 | v < n }, n: i64) -> i64 { k }

fn main() {}
// error: cannot find value `n` in this refinement
