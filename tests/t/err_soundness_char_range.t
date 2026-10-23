// The logic knows a character's range, but no more.
fn f(c: char) -> { v: u32 | v < 0xD800 } { c as u32 }

fn main() {}
// error: can't prove `v < 55296` for the result of `f`
