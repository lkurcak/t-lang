// Not even whole: in the logic, `==` would say the same bits, which isn't
// the program's `==`: `0.0 == -0.0`, but they aren't the same bits.
fn f(a: f64, b: { b: f64 | b == a }) -> f64 { b }
// error: a refinement can't name `b`, of type `f64`: floats aren't in its logic

fn main() {}
