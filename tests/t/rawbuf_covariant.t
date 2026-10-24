// A buffer owns its `T`s, as a box does, so it's covariant in `T`: one of
// `&'static i64`s will do as one of shorter-lived references, though its
// type has no fields to say so.
use alloc::raw::RawBuf;

fn shorten<'a>(b: RawBuf<&'static i64>) -> RawBuf<&'a i64> { b }

fn main() with alloc {
    let n = 7;
    let mut b: RawBuf<&i64> = shorten(RawBuf::new(1));
    // SAFETY: slot 0 is uninitialised; then it holds `&n`, which is read
    // out before the buffer is freed.
    unsafe {
        b.write(0, &n);
        print(*b.read(0)); // expect: 7
        b.free();
    }
}
