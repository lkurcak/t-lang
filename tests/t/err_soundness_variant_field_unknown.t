// Nothing about what's made from an unknown is known but what's proved.
#[derive(Copy, Discard)]
enum Key { Digit({ d: u8 | d < 10 }), Other }

fn make(n: u8) -> Key { Key::Digit(n) }

fn main() {
    let _k = make(3);
}
// error: can't prove `d < 10` for field 0 of `Key::Digit`
