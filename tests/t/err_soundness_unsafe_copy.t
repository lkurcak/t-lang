// A struct with an `Unsafe` in it can't be `Copy`: a copy would be a
// second cell, which a change to the first doesn't change.
use core::cell::Unsafe;

#[derive(Copy, Discard)]
// error: `Copy`
struct Flag { set: Unsafe<bool> }

fn main() {}
