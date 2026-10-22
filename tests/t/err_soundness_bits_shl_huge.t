// 2^128 is more than the logic's constants hold: the checker mustn't take
// `x << 128` for `x`, as a shift that wraps around would.
fn g(x: { v: u8 | v == 1 }) -> u8 { x << 128 }

fn main() {}
// error: `<<` may panic, but `g` doesn't say `with panic`
