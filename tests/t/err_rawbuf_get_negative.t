// There's no slot before the first, and that half of an index's range is
// the checker's to prove, not an obligation: `i` is `{ i: int | i >= 0 }`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: wrong: there's no slot -1.
    unsafe {
        print(*b.get(-1));
        b.free();
    }
}
// error: can't prove `i >= 0` for argument `i` of `get`
