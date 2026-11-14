// What's in a `Cell` a closure gave back can change through a `&`: what
// the closure knew of it when it gave it isn't what's there after (lemma
// A4 of docs/60).
use core::cell::Cell;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() {
    let c: Cell<u32> = call(|| Cell::new(5));
    c.set(7);
    let v: { v: u32 | v == 5 } = c.get();
    let _ = v;
}
// error: can't prove `v == 5` for `v`
