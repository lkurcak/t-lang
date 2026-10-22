// Reading a slot that holds nothing breaks `read`'s obligation. The
// checker can't see that; the interpreter does, and blames the `unsafe`
// block that said it was kept.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: wrong: slot 1 was never written.
    unsafe {
        b.write(0, 1);
        print(b.read(1));
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::read` of slot 1, which is uninitialised
