// A buffer isn't `Discard`: its memory, and whatever is in it, must be
// given back, by `free`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let _b: RawBuf<i64> = RawBuf::new(4);
}
// error: `_b` is never used up, and its type, `RawBuf<i64>`, isn't `Discard`
