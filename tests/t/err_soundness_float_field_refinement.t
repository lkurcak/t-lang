// A struct's field of a float can't be refined either.
struct Unit { x: { x: f64 | x <= 1 } }
// error: a refinement can't name `x`, of type `f64`: floats aren't in its logic

fn main() {}
