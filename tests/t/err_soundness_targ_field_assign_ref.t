// Nor be assigned to such a field.
use core::cell::Cell;

#[derive(Discard)]
struct Holder<'a> { c: &'a Cell<u8> }

fn poke(h: Holder) { h.c.set(9) }

fn main() {
    let plain: Cell<u8> = Cell::new(0);
    let c: Cell<{ t: u8 | t < 3 }> = Cell::new(1);
    let mut h = Holder { c: &plain };
    h.c = &c;
    poke(h);
}
// error: can't prove `t < 3` of what may be written through `h`
