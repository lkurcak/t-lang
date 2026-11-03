// `get` copies what's in the cell, so it needs `Copy`.
use alloc::boxed::Box;
use core::cell::Cell;

fn main() with alloc {
    let c = Cell::new(Box::new(1i64));
    let b = c.get();
}
// error: Copy
