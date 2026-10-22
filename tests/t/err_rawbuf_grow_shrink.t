// `grow` only grows: shrinking could lose the values in the slots cut off.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(4);
    b.grow(2);
    // SAFETY: nothing was written.
    unsafe { b.free() };
}
// error: runtime error: `grow` to 2 slots, but it has 4
