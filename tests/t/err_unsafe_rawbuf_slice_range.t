// A slice must be of the buffer's slots: `hi <= cap()`, and `lo <= hi`,
// which `slice`'s refinement says, so the checker proves it.
use alloc::raw::RawBuf;

fn main() with alloc {
    let b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: wrong: there's no slot 1.
    let s = unsafe { b.slice(1, 2) };
    let _v = s.len();
    // SAFETY: every slot is uninitialised.
    unsafe { b.free() };
}
// error: can't prove `h >= lo && h <= self.cap()` for argument `hi` of `slice`
