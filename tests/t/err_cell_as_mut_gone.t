// A `Cell` lends nothing out, not even unsafely: what's in it is only
// copied or moved out. Lending is `Unsafe`'s, which a cell is built on.
use core::cell::Cell;

fn main() {
    let c = Cell::new(1u32);
    // SAFETY: nothing else reaches it.
    let a = unsafe { c.as_mut() };
    // error: has no method `as_mut`
    *a = 2;
}
