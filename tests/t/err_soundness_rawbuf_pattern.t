// Taking a buffer apart with a pattern would use it up without giving its
// memory back.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let b: RawBuf<i64> = RawBuf::new(4);
    let RawBuf {} = b;
}
// error: `RawBuf` is intrinsic
