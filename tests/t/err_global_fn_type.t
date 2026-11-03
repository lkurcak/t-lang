// Nor can a function that uses a static be a `fn` without `global`.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn read() -> u32 with global { COUNT.load() }

fn main() {
    let f: fn() -> u32 = read;
// error: expected `fn() -> u32`, found `fn() -> u32 with global`
    let _ = f;
}
