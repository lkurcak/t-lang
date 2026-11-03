// `main` says its effects, `global` among them.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main() with {} {
    let _ = COUNT.load();
// error: `COUNT` is a static, but `main` doesn't say `with global`
}
