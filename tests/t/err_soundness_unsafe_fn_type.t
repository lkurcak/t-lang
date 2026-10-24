// A function type without `with unsafe` can't hold an unsafe function:
// calling it would be unsafe with nothing to say so.
use alloc::raw::RawBuf;

fn take(b: &mut RawBuf<i64>) -> i64 with unsafe { b.read(0) }

fn main() with {alloc, panic} {
    let f: fn(&mut RawBuf<i64>) -> i64 = take;
    let mut b: RawBuf<i64> = RawBuf::new(1);
    let _v = f(&mut b);
    // SAFETY: the slot is uninitialised.
    unsafe { b.free() };
}
// error: expected `fn(&mut RawBuf<i64>) -> i64`, found `fn(&mut RawBuf<i64>) -> i64 with unsafe`
