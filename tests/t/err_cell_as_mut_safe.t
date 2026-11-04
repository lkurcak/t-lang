// Lending what's in a cell as `&mut` from a `&` is unsafe: two such loans
// would be two `&mut`s to one value.
use core::cell::Cell;

fn main() {
    let c = Cell::new(1u32);
    let a = c.as_mut();
    // error: unsafe
    *a = 2;
}
