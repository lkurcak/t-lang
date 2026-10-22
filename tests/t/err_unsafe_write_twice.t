// Writing a slot that holds a value would throw that value away.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: wrong: slot 0 is written twice.
    unsafe {
        b.write(0, 1);
        b.write(0, 2);
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::write` of slot 0, which is initialised
