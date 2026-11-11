// Lent to a call that can write anything, the field's refinement is proved
// again after it: and can't be.
#[derive(Copy, Discard)]
enum Key { Digit({ d: u8 | d < 10 }), Other }

fn big(r: &mut u8) {
    *r = 200;
}

fn main() {
    let mut k = Key::Digit(3);
    match &mut k {
        Key::Digit(d) => big(d),
        Key::Other => {},
    }
}
// error: can't prove `d < 10`
