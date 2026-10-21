// A parameter's refinement can only name the parameters before it.
fn f(k: { v: int | v < n }, n: int) -> int { k }

fn main() {}
// error: cannot find value `n` in this refinement
