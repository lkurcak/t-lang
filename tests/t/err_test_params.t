// A test is run as a program of its own: nothing could give it an argument.
fn main() {}

#[test]
fn takes(x: u8) { assert(x == 1); }
// error: a test takes nothing
