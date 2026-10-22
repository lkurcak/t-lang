// A struct holding a `&mut [T]` is invariant in `T`, as the slice is:
// `SplitMut`'s halves could be written through.
use core::slice::SplitMut;

fn shorter<'a, 'b>(s: SplitMut<'b, &'static i64>) -> SplitMut<'b, &'a i64> { s }

fn main() {}
// error: lifetime may not live long enough
