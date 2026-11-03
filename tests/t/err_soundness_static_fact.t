// Nor of a static's atomic: it may have changed by the next load.
use core::sync::Atomic;

static A: Atomic<u32> = Atomic::new(1);

fn one(_x: { x: u32 | x == 1 }) {}

fn main() with global {
    if A.load() == 1 {
        one(A.load());
// error: can't prove `x == 1` for argument `_x` of `one`
    }
}
