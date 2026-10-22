// There's no slot before the first, and that half of an index's range is
// the checker's to see, not an obligation: `lo` is a `usize`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: wrong: there's no slot -1.
    unsafe {
        print(b.slice(-1, 0).len());
        b.free();
    }
}
// error: `-1` doesn't fit in a `usize`, whose values are 0 to 4294967295
