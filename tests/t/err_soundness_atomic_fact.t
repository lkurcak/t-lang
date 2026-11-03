// What an atomic holds is opaque to the checker's logic: another thread
// may change it between any two steps, so nothing is known of a load.
use core::sync::Atomic;

fn one(_x: { x: u32 | x == 1 }) {}

fn main() {
    let a = Atomic::new(1u32);
    a.store(1);
    one(a.load());
// error: can't prove `x == 1` for argument `_x` of `one`
}
