// An `unsized` parameter may have no size, so it's never a value: not a
// parameter's type.
fn f<unsized D>(x: D) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
