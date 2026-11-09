// A const parameter is a length, which has no size to have or not.
fn f<unsized const N: usize>() {}

fn main() {}
// error: expected a type parameter after `unsized`, found `const`
