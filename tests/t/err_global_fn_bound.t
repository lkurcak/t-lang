// A bound with no `with` is a closure with no effects: none that uses a
// static.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn call<F: Fn() -> u32>(f: F) -> u32 { f() }

fn main() with global {
    let _ = call(|| COUNT.load());
// error: global
}
