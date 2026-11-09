// A struct's field whose type has no type parameter for it can't hold a
// reference to a place whose refinements are where something can write:
// the struct's type wouldn't say them, so whatever's given it couldn't
// keep them.
use core::cell::Cell;

#[derive(Discard)]
struct Holder<'a> { c: &'a Cell<u8> }

fn poke(h: Holder) { h.c.set(9) }

fn main() {
    let c: Cell<{ t: u8 | t < 3 }> = Cell::new(1);
    poke(Holder { c: &c });
}
// error: can't prove `t < 3` of what may be written through field `c` of `Holder`
