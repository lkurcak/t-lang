// A buffer can't have fewer than no slots: how many is a `usize`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<i64> = RawBuf::new(-1);
    // SAFETY: nothing was written.
    unsafe { b.free() };
}
// error: `-1` doesn't fit in a `usize`, whose values are 0 to 4294967295
