// Nor what an array holds.
fn f<unsized D>(x: &[D; 2]) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
