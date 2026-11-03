// A `&mut` to what's in a static would be a second way to it, unshared.
use core::sync::Atomic;

#[derive(Discard, Sync)]
struct S { n: u32, a: Atomic<u32> }

static X: S = S { n: 1, a: Atomic::new(0) };

fn main() with global {
    let r = &mut X.n;
// error: cannot borrow `X.n` as mutable: it's in a static, which is only ever shared
    *r = 2;
}
