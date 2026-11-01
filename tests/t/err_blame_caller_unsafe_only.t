// `unsafe` isn't an abort: blaming the caller would change nothing.
#[blame_caller]
#[allow(unused_effects)]
fn f() with unsafe {}

fn main() {}
// error: `#[blame_caller]` on a function that can't abort
