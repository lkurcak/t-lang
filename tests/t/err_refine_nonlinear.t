fn f(k: { v: int | v * v < 10 }) -> int { k }

fn main() {}
// error: a refinement can only multiply by a constant: `v * v` isn't linear
