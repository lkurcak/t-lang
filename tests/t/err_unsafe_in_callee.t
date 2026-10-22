// An obligation broken in a function that says `with unsafe` is its
// caller's block's to answer for: that's where it was said to be kept.
use alloc::raw::RawBuf;

fn second(b: &RawBuf<int>) -> int with unsafe { *b.get(1) }

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: wrong: `second` needs slot 1 written.
    unsafe {
        b.write(0, 1);
        print(second(&b));
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::get` of slot 1, which is uninitialised
