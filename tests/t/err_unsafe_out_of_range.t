// Slot `cap()` is one past the end.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: wrong: there are only slots 0 and 1.
    unsafe {
        b.write(2, 1);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::write` of slot 2, but it has 2 slots
