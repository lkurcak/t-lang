// Which of `&` and `|` comes first is said with parentheses.
fn f(a: u8, b: u8, c: u8) -> u8 { a & b | c }

fn main() {}
// error: use parentheses
