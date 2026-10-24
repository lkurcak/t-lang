// Core's `Int` is sealed by what it is, not by what it's called here.
use core::num::Int as Number;

#[derive(Copy, Discard, Eq)]
struct S {}

impl Number for S {}

fn main() {}
// error: `Int` is built in: only the integer types implement it
