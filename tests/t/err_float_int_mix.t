// An integer literal is never a float.
fn f(x: f64) -> f64 { x + 1 }
// error: expected `f64`, found `{integer}`

fn main() {}
