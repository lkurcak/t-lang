fn f(x: f64) -> f64 { x << 1 }
// error: `<<` works on integers, not on `f64`

fn main() {}
