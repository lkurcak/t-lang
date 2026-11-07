// An `Unsafe<&'static str>` is no `Unsafe<&'a str>`: through a `&` to it,
// `as_mut` could put in a reference that dies before it's read.
use core::cell::Unsafe;

fn shorten<'a>(c: &'a Unsafe<&'static str>) -> &'a Unsafe<&'a str> { c }

fn main() {}
// error: lifetime may not live long enough
