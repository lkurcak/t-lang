// An obligation broken in a function that says `with unsafe` is its
// caller's block's to answer for: that's where it was said to be kept.
use alloc::raw::RawBuf;

fn second(b: &RawBuf<i64>) -> usize with unsafe {
    if b.cap() > 1 { b.slice(1, 2).len() } else { 0 }
}

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: wrong: `second` needs slot 1 written.
    unsafe {
        b.write(0, 1);
        let _v = second(&b);
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::slice` of slots 1..2, but slot 1 is uninitialised
