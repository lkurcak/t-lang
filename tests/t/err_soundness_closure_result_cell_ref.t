// What a closure gives that a `Cell` behind it holds can change after,
// through a `&`: where the type is invariant, what the closure knew there
// isn't what's there (lemma A4 of docs/60).
use core::cell::Cell;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() {
    let c: Cell<u32> = Cell::new(5);
    let r: &Cell<u32> = call(|| &c);
    r.set(7);
    let v: { v: u32 | v == 5 } = r.get();
    let _ = v;
}
// error: can't prove `v == 5` for `v`
