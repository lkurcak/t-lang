// Saying a buffer may be thrown away would leak it, and it's not this
// program's to say: `Discard` is `core`'s and `RawBuf` is `alloc`'s.
use alloc::raw::RawBuf;

impl<T> Discard for RawBuf<T> {}

fn main() with {alloc, panic} {
    let _b: RawBuf<i64> = RawBuf::new(4);
}
// error: `Discard` is `core`'s and `RawBuf` is `alloc`'s, so only `core` or `alloc` can implement one for the other
