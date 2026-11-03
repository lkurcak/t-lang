// A static can't be moved out of: it's there for the whole run.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main() with global {
    let n = COUNT;
// error: cannot move out of `COUNT`: it's in a static, which is there for the whole run
    let _ = n.into_inner();
}
