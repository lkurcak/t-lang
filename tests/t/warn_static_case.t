// A static's name is upper case, as a const's is.
use core::sync::Atomic;

static count: Atomic<u32> = Atomic::new(0);
// warning: static `count` should have an upper case name

fn main() with global {
    let _ = count.load();
}
