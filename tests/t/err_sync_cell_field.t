// A struct with a cell in it isn't `Sync`: two threads could change the
// cell at once through two `&`s.
use core::cell::Cell;

#[derive(Discard, Sync)]
// error: `Counter` can't be `Sync`: its field `n` holds a `Cell<u32>`, which isn't
struct Counter { n: Cell<u32> }

fn main() {}
