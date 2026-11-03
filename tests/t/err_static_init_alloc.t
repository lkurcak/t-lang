// A static's value is worked out while compiling, as a const's is.
use alloc::boxed::Box;
use core::sync::Atomic;

fn start() -> u32 with alloc {
    let b = Box::new(3u32);
    *b
}

static COUNT: Atomic<u32> = Atomic::new(start());
// error: a static's initializer can't have the effect `alloc`

fn main() {}
