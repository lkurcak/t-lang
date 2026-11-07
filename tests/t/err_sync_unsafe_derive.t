// A struct with an `Unsafe` in it isn't `Sync`: two threads could change
// what's in it at once through two `&`s. Only an `unsafe impl` says it is.
use core::cell::Unsafe;

#[derive(Discard, Sync)]
// error: `Counter` can't be `Sync`: its field `n` holds a `Unsafe<u32>`, which isn't
struct Counter { n: Unsafe<u32> }

fn main() {}
