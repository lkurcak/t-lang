// What the closure is called with is a `T` the callee may make (`T: Int`),
// not the argument: so the closure's parameter is any number, and so is
// what it gives back.
use core::num::{Int, greatest};

fn app<T: Int, R, F: FnOnce(T) -> R + Discard>(_x: T, f: F) -> R { f(greatest::<T>()) }

fn main() {
    let x: { v: u32 | v < 10 } = 4;
    let c: { v: u32 | v < 10 } = app(x, |z| z);
    let _ = c;
}
// error: can't prove `v < 10`
