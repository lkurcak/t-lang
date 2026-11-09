// Nor what a function gives.
fn f<'a, unsized D>(x: &'a D) -> Option<D> { None }

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
