// Nor what a slice holds: its values are values.
fn f<unsized D>(x: &[D]) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
