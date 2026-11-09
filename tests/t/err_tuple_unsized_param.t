// Nor an `unsized` type parameter.
fn f<unsized D>(_t: &(i64, D)) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
