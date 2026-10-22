// A slice must be of slots that hold values.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: slot 0 is uninitialised.
    unsafe { b.write(0, 1) };
    // SAFETY: wrong: slot 1 holds nothing.
    let s = unsafe { b.slice(0, 2) };
    print(s.len());
    // SAFETY: slot 0 holds the 1; then every slot is uninitialised.
    unsafe {
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::slice` of slots 0..2, but slot 1 is uninitialised
