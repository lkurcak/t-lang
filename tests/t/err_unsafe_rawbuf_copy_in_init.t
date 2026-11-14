// `copy_in` copies into slots that hold nothing: one that holds a value
// breaks its obligation, as `write` does.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: slot 1 is uninitialised.
    unsafe { b.write(1, 5) };
    // SAFETY: wrong: slot 1 holds the 5.
    unsafe { b.copy_in(0, [1i64, 2].as_slice()) };
    // SAFETY: both hold values; then every slot is uninitialised.
    unsafe {
        let _ = b.read(0);
        let _ = b.read(1);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::copy_in` of slots 0..2, but slot 1 is initialised
