// Nor an effect parameter.
fn f<unsized effect e>() {}

fn main() {}
// error: expected a type parameter after `unsized`, found `effect`
