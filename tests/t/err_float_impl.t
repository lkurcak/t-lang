// `Float` is sealed: only `f32` and `f64` are floats.
#[derive(Copy)]
struct Half { bits: u16 }
impl Float for Half {}
// error: `Float` is sealed: only its own package, `core`, can implement it

fn main() {}
