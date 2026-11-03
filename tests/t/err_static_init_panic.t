// A static's initializer that panics is an error while compiling.
use core::sync::Atomic;

fn start() -> u32 with panic { panic("no start") }

static COUNT: Atomic<u32> = Atomic::new(start());
// error: no start

fn main() with global {
    let _ = COUNT.load();
}
