// `as u8` of a character is checked like any `as`.
fn f(c: char) -> u8 with {} { c as u8 }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
