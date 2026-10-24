// A function that says `with unsafe` passes its obligations on: calling
// it is unsafe too.
use alloc::raw::RawBuf;

fn first(b: &RawBuf<i64>) -> usize with unsafe { b.slice(0, 1).len() }

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised, and there is one.
    unsafe { b.write(0, 7) };
    let _v = first(&b);
    // SAFETY: slot 0 holds the 7, and is the only slot.
    unsafe {
        b.read(0);
        b.free();
    }
}
// error: `first` is unsafe, but it isn't in an `unsafe` block
