// Making a variant proves its fields' refinements.
#[derive(Copy, Discard)]
enum Key { Digit({ d: u8 | d < 10 }), Other }

fn main() {
    let _k = Key::Digit(10);
}
// error: can't prove `d < 10` for field 0 of `Key::Digit`
