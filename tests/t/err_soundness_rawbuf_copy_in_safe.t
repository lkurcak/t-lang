// `copy_in` is unsafe: it's only sound into slots that hold nothing.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    b.copy_in(0, [1i64, 2].as_slice());
    // SAFETY: wrong anyway, as both slots hold values.
    unsafe { b.free() };
}
// error: `copy_in` is unsafe, but it isn't in an `unsafe` block
