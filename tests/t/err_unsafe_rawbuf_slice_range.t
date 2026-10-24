// A slice must be of the buffer's slots: `hi <= cap()`, and `lo <= hi`.
use alloc::raw::RawBuf;

fn main() with alloc {
    let b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: wrong: there's no slot 1.
    let s = unsafe { b.slice(1, 2) };
    print(s.len());
    // SAFETY: every slot is uninitialised.
    unsafe { b.free() };
}
// error: unsafe code broke its obligation: `RawBuf::slice` of slots 1..2, but the buffer has 1
