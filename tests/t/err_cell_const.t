// A const is frozen: nothing in it can change through a `&`, so no cell.
use core::cell::Cell;

#[derive(Discard)]
struct Counter { n: Cell<i64> }

const START: Counter = Counter { n: Cell::new(0) };

fn main() {}
// error: a const can't be of type
