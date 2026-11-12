// A const parameter is an unknown: `i / N` isn't linear.
fn f<const N: usize>(a: [u8; N], i: { v: usize | v / N == 0 }) {}

fn main() {}
// error: a refinement can use `/` only by a constant: `N` isn't one
