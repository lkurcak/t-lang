// A `usize` may be 32 bits wide, so a `u64` needn't fit in one, though
// on this machine it would.
fn f(n: u64) -> usize { n as usize }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
