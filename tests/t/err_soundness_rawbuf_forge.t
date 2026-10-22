// Only `RawBuf::new` makes a buffer: a literal would be one with no
// memory behind it.
use alloc::raw::RawBuf;

fn main() {
    let b: RawBuf<int> = RawBuf {};
    // SAFETY: there's nothing in it.
    unsafe { b.free() };
}
// error: `RawBuf` is intrinsic
