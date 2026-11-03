// What's put in a cell must live as long as the cell's type says.
use core::cell::Cell;

fn keep(c: &Cell<&'static str>, s: &str) { c.set(s); }

fn main() {}
// error: lifetime may not live long enough
