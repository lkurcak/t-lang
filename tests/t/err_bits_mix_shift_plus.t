// A shift on the left of `+` needs parentheses too: `(a << 1) + b`.
fn f(a: u8, b: u8) -> u8 with panic { a << 1 + b }

fn main() {}
// error: use parentheses
