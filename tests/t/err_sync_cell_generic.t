// Nor is a cell, whatever's in it.
use core::cell::Cell;

fn sync<T: Sync>(_t: &T) {}

fn main() {
    let c = Cell::new(1u8);
    sync(&c);
// error: `sync` needs its `T` to implement `Sync`, and `Cell<u8>` doesn't
}
