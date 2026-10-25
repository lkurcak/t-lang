// A const parameter stands for a `usize`, not a type.
fn f<const N: usize>(x: N) {}

fn main() {}
// error: `N` is a const parameter, not a type
