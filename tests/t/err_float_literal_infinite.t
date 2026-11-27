// The nearest `f32` of 1e39 is an infinity, which a literal can't be.
fn f() -> f32 { 1e39 }
// error: `1e39` is too big for an `f32`: its nearest value is an infinity

fn main() {}
