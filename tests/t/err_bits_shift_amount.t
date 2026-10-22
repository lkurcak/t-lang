// A shift amount is a `u32`, whatever is shifted.
fn f(x: u64, n: u8) -> u64 { x >> n }

fn main() {}
// error: expected `u32`, found `u8`
