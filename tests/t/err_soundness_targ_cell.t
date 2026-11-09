// What's in a cell can change through a `&`: a cell of numbers below 3
// can't be given as a `&Cell<u8>`, which `poke` could set to 9.
use core::cell::Cell;

fn poke(c: &Cell<u8>) { c.set(9) }

fn main() {
    let c: Cell<{ t: u8 | t < 3 }> = Cell::new(1);
    poke(&c);
}
// error: can't prove `t < 3` of what `poke` may write behind argument `c`
