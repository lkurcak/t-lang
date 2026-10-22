fn f(k: { v: i64 | v * v < 10 }) -> i64 { k }

fn main() {}
// error: a refinement can only multiply by a constant: `v * v` isn't linear
