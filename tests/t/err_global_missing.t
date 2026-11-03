// An exported function says it uses a static: `global` is part of what it
// does, and its callers' business.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

export fn count() -> u32 { COUNT.load() }
// error: `COUNT` is a static, but `count` doesn't say `with global`

fn main() {
    let _ = count();
}
