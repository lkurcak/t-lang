// Floats aren't in the logic: a refinement can't name one, which would
// let it prove what IEEE 754's `<` doesn't say of a NaN.
fn f(x: { x: f64 | x > 0 }) -> f64 { x }
// error: a refinement can't name `x`, of type `f64`: floats aren't in its logic

fn main() {}
