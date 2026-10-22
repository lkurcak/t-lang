// A refinement is of its type's values: `{ n: u8 | n > 200 }` can't be
// given 300, and every `u8` is at most 255, so `n + 1` can't overflow
// only if `n < 255`.
fn f(n: { n: u8 | n > 200 }) -> u8 { n + 1 }

fn main() {}
// error: `+` may panic, but `f` doesn't say `with panic`
