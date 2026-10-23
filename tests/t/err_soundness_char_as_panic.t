// Not every `u32` is a character: `as char` panics unless it's proved one.
fn f(n: u32) -> char { n as char }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
