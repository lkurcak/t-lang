// A function type's parameters are values.
fn f<unsized D>(g: fn(D) -> u8) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
