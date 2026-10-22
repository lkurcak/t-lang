// No type is widened for you: say `a as u16`.
fn f(a: u8, b: u16) -> u16 with panic { a + b }

fn main() {}
// error: expected `u8`, found `u16`
