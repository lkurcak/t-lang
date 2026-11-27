// Of a type parameter `F: Float`, a literal must be finite in an `f32`.
fn f<F: Float>() -> F { 1e300 }
// error: `1e300` is too big for `F`, which may be an `f32`: its nearest value is an infinity

fn main() {}
