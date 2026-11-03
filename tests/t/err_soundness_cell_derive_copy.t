// Nor is a struct with a cell in it.
use core::cell::Cell;

#[derive(Copy, Discard)]
struct S { c: Cell<i64> }

fn main() {}
// error: Copy
