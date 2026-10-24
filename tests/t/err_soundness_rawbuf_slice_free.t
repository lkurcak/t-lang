// `free` gives the memory back, so a slice from `slice` can't live past
// it, nor `read` take a value out from under it.
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised, then holds 5, which is read out
    // before the buffer is freed.
    unsafe {
        b.write(0, 5);
        let s = b.slice(0, 1);
        b.read(0);
        print(s.len());
        b.free();
    }
}
// error: cannot borrow `b` as mutable while `b` is borrowed
