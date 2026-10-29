// 1 << 64 doesn't fit in a `u64`.
fn f(x: { v: u64 | v == 1 }) -> u64 with {} { x << 64 }

fn main() {}
// error: `<<` may panic, but `f` doesn't say `with panic`
