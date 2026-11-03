// Nor can a const's: its value mustn't depend on what a static holds.
use core::sync::Atomic;

static A: Atomic<u32> = Atomic::new(1);
const B: u32 = A.load();
// error: a const's initializer can't have the effect `global`

fn main() {}
