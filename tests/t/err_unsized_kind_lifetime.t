// `unsized` is for type parameters.
fn f<unsized 'a>(x: &'a u8) {}

fn main() {}
// error: expected a type parameter after `unsized`, found `'a`
