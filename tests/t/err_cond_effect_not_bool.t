// A condition is a `bool`.
fn f(x: u8) -> u8 with panic if x { x }

fn main() {}
// error: expected `bool`
