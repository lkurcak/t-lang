// A buffer can't have fewer than no slots.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<int> = RawBuf::new(-1);
    // SAFETY: nothing was written.
    unsafe { b.free() };
}
// error: can't prove `c >= 0` for argument `cap` of `new`
