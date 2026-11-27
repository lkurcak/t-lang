// Not every `f64` is an `f32`: 0.1 isn't, nor anything past the greatest.
fn f(x: f64) -> f32 with {} { x as f32 }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
