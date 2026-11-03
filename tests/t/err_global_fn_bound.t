// A bound with no `with` is a closure with no effects: none that uses a
// static.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn call<F: Fn() -> u32 + Discard>(f: F) -> u32 { f() }

fn main() with global {
    let _ = call(|| COUNT.load());
// error: this closure may use a static, but `call` needs calls of it to fit `fn() -> u32`, which can't use a static
}
