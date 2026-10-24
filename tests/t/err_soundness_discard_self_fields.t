// In its own `discard`, what's left of `self` is dropped field by field:
// a buffer isn't `Discard`, so it must be freed.
use alloc::raw::RawBuf;

struct Buf { raw: RawBuf<i64> }

impl Discard for Buf {
    fn discard(self) {}
}

fn main() with alloc {
    let _b = Buf { raw: RawBuf::new(1) };
}
// error: `self.raw` is never used up
