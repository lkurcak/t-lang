// Nor a cell itself.
use core::cell::Cell;

const COUNT: Cell<i64> = Cell::new(0);

fn main() {}
// error: a const is frozen
