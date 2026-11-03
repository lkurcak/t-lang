// A static's value is worked out while compiling, so it's plain data, and
// atomics: no box, no reference but to a literal.
use alloc::boxed::Box;
use core::sync::Atomic;

#[derive(Discard, Sync)]
struct Held { b: Box<u8>, n: Atomic<u8> }

static H: Held = Held { b: Box::new(1), n: Atomic::new(0) };
// error: a static can't be of type `Held`

fn main() {}
