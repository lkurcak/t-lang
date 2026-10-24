// Freeing a buffer with a value still in it would lose the value, which
// may be one that must be used up.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(3);
    // SAFETY: wrong: slot 2 still holds a value at `free`.
    unsafe {
        b.write(2, 1);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::free` with slot 2 initialised
