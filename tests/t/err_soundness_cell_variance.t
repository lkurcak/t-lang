// A cell of `&'static str` is no cell of `&'a str`: through a `&` to it,
// `set` would put in a reference that dies before the cell is read.
use core::cell::Cell;

fn shorten<'a>(c: &'a Cell<&'static str>) -> &'a Cell<&'a str> { c }

fn main() {}
// error: lifetime may not live long enough
