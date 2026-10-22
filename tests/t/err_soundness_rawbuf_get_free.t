// `free` gives the memory back, so a reference from `get` can't live past
// it.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised, then holds 5, which is read out
    // before the buffer is freed.
    unsafe {
        b.write(0, 5);
        let r = b.get(0);
        b.read(0);
        print(*r);
        b.free();
    }
}
// error: cannot borrow `b` as mutable while `b` is borrowed
