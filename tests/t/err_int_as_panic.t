// An `i64` needn't fit in a `u8`.
fn f(n: i64) -> u8 with {} { n as u8 }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
