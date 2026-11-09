// A tuple is `Sync` only if its values are: sharing this one would share
// the cell.
use core::cell::Cell;

fn share<T: Sync>(_t: &T) {}

fn main() {
    let t = (Cell::new(1u32), 2u32);
    share(&t);
}
// error: `share` needs its `T` to implement `Sync`, and `(Cell<u32>, u32)` doesn't; that needs `Cell<u32>: Sync`
