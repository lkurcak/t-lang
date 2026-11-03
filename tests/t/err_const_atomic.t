// A const is frozen, so no atomic either.
use core::sync::Atomic;

const COUNT: Atomic<u32> = Atomic::new(0);
// error: a const is frozen

fn main() {}
