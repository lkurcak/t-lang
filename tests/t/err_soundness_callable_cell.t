// A closure's value called after what it reads through a `Cell` changed:
// what its walk, when it was made, could know of the cell isn't what's
// there now.
use core::cell::Cell;

fn main() {
    let c: Cell<u32> = Cell::new(1);
    let f = || c.get();
    c.set(0);
    let x: { v: u32 | v > 0 } = f();
    let _ = x;
}
// error: can't prove `v > 0` for `x`
