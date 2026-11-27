// `F as f32` may narrow: `F` may be an `f64`.
fn f<F: Float>(x: F) -> f32 with {} { x as f32 }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
