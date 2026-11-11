// A generic function writes through the reference what it's given: of the
// type the reference says, refinement and all.
#[derive(Copy, Discard)]
enum Key { Digit({ d: u8 | d < 10 }), Other }

fn put<T: Discard>(r: &mut T, v: T) { *r = v; }

fn main() {
    let mut k = Key::Digit(3);
    match &mut k {
        Key::Digit(d) => put(d, 10),
        Key::Other => {},
    }
}
// error: can't prove `d < 10`
