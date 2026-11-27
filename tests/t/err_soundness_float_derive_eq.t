// Nor is a struct that holds one, field by field.
#[derive(Eq)]
struct P { x: f64 }
// error: `P` can't be `Eq`: its field `x` holds an `f64`, which isn't

fn main() {}
