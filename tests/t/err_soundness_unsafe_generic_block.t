// An `unsafe` block takes on the obligations of the operations in it,
// not of what a type parameter turns out to be: `call` can't know what
// `f`'s are. So `f`'s effects `e` stay `call`'s, and its caller, which
// passes an unsafe closure, must keep them.
use alloc::raw::RawBuf;

/// Slot `i`'s value, moved out, if there's a slot `i`.
fn get(b: &mut RawBuf<i64>, i: usize) -> i64 with unsafe {
    if i < b.cap() { b.read(i) } else { 0 }
}

fn call<effect e, F: FnOnce() -> i64 with e>(f: F) -> i64 with e {
    // SAFETY: nothing here is unsafe but `f`, whose obligations aren't ours.
    unsafe { f() }
}

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    let _v = call(|| get(&mut b, 0));
    // SAFETY: there's nothing in the slot to leave behind.
    unsafe { b.free() };
}
// error: `call` is unsafe, but it isn't in an `unsafe` block
// warning: this `unsafe` block has nothing unsafe in it
