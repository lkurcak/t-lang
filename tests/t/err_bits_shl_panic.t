// `<<` panics if the result doesn't fit: 200 << 1 is 400.
fn f(x: u8) -> u8 { x << 1 }

fn main() {}
// error: `<<` may panic, but `f` doesn't say `with panic`
