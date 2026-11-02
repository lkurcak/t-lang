// Nor functions: a value of a function type isn't data.
fn one() -> u8 { 1 }

const F: fn() -> u8 = one;
// error: a const can't be of type `fn() -> u8`

fn main() {}
