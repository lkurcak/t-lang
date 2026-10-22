// A buffer isn't `Copy`: freeing it moves it, so it can't be freed twice.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<i64> = RawBuf::new(4);
    // SAFETY: nothing was written, so every slot is uninitialised.
    unsafe {
        b.free();
        b.free();
    }
}
// error: use of moved value `b`
