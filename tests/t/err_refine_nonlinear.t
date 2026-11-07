fn f(k: { v: i64 | v * v * v < 10 }) -> i64 { k }

fn main() {}
// error: a refinement can multiply at most two unknowns: `v * v * v` multiplies three
