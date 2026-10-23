// Surrogates are in range, but not characters.
fn f(n: { v: u32 | v <= 0xDFFF }) -> char { n as char }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
