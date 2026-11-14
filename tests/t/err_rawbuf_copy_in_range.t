// What's copied in must fit: slots 1..3 of a buffer of 2 aren't there.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: no slot is initialised.
    unsafe { b.copy_in(1, [1i64, 2].as_slice()) };
    // SAFETY: nothing was copied in.
    unsafe { b.free() };
}
// error: can't prove `at + s.len() <= self.cap()` for argument `src` of `copy_in`
