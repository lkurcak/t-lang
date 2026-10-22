// There's no slot before the first, and that half of an index's range is
// the checker's to prove, not an obligation: `lo` is `{ l: int | l >= 0 }`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: wrong: there's no slot -1.
    unsafe {
        print(b.slice(-1, 0).len());
        b.free();
    }
}
// error: can't prove `l >= 0` for argument `lo` of `slice`
