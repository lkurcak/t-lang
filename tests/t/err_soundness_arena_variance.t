// An arena of `&'static str` is no arena of `&'a str`: through a `&` to
// it, `alloc` would put in a reference that dies before the arena does.
use alloc::arena::Arena;

fn shorten<'a>(a: &'a Arena<&'static str>) -> &'a Arena<&'a str> { a }

fn main() {}
// error: lifetime may not live long enough
