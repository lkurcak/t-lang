// `-x` of a `u8` fits only when `x` is 0.
fn f(x: u8) -> u8 { -x }

fn main() {}
// error: `-` may panic, but `f` doesn't say `with panic`
