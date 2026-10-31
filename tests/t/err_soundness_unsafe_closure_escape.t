// A closure's effects are its body's, and an `unsafe` block around the
// closure doesn't take on what calling it later does: the obligations are
// kept, or not, where it's called.
use alloc::raw::RawBuf;

/// Slot `i`'s value, moved out, if there's a slot `i`.
fn get(b: &mut RawBuf<i64>, i: usize) -> i64 with unsafe {
    if i < b.cap() { b.read(i) } else { 0 }
}

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: the closure isn't called here.
    let mut f = unsafe { |b: &mut RawBuf<i64>| get(b, 0) };
    let _v = f(&mut b);
    // SAFETY: the slot is uninitialised.
    unsafe { b.free() };
}
// error: `f` is unsafe, but it isn't in an `unsafe` block
// warning: this `unsafe` block has nothing unsafe in it
