// Slot `cap()` is one past the end, which `write`'s refinement rules out.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: wrong: there are only slots 0 and 1.
    unsafe {
        b.write(2, 1);
        b.free();
    }
}
// error: can't prove `i < self.cap()` for argument `i` of `write`
