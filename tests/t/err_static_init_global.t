// A static's initializer can't read another static: what's in one is only
// known as the program runs.
use core::sync::Atomic;

static A: Atomic<u32> = Atomic::new(1);
static B: Atomic<u32> = Atomic::new(A.load());
// error: a static's initializer can't have the effect `global`

fn main() {}
