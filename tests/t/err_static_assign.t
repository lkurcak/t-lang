// A static isn't assigned to: it changes only through what's in it.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main() with global {
    COUNT = Atomic::new(1);
// error: cannot assign to `COUNT`, a static: it's only ever shared
}
