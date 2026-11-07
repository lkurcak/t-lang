// A list owns its values, as a box does, so a list of cells isn't frozen
// either: what's in the cells could change through a `&` to the const.
use alloc::list::List;
use core::cell::Cell;

const CELLS: List<Cell<i32>> = List::new();
// error: a const is frozen

fn main() {}
