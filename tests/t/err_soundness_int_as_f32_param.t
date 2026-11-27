// Of a type parameter `F: Float`, `as` must be exact for an `f32`, the
// narrower: an `i32` of 2^24 + 1 isn't one, so it isn't proved, though it
// would be for an `f64`.
fn f<F: Float>(n: i32) -> F with {} { n as F }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
