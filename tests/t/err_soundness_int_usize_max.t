// Nor does the checker know a `usize` is 64 bits wide: `+ 1` of a number
// that would fit a 64-bit `usize` may not fit a 32-bit one.
fn f(n: { n: usize | n < 4294967295 }) -> usize { n + 1 }

fn g(n: { n: usize | n < 4294967296 }) -> usize with {} { n + 1 }

fn main() {}
// error: `+` may panic, but `g` doesn't say `with panic`
