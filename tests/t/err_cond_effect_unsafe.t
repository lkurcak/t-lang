// `unsafe` is a promise the caller keeps, not something the body does: it
// can't be conditional (docs/09-unsafe.md).
fn f(x: u8) -> u8 with unsafe if x > 3 { x }

fn main() {}
// error: `unsafe` can't have a condition: it says what the caller must keep, not what the call does
