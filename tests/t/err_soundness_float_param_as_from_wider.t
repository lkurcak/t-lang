// `f64 as F` may narrow: `F` may be an `f32`.
fn f<F: Float>(x: f64) -> F with {} { x as F }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
