// What's written through a `&mut` bound to a refined variant field must
// have its refinement.
#[derive(Copy, Discard)]
enum Key { Digit({ d: u8 | d < 10 }), Other }

fn main() {
    let mut k = Key::Digit(3);
    match &mut k {
        Key::Digit(d) => *d = 10,
        Key::Other => {},
    }
}
// error: can't prove `d < 10`
