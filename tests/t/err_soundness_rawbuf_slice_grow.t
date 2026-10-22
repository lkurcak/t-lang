// `grow` may move the slots, so a slice of them can't live past it:
// `slice` borrows the buffer, and `grow` needs it `&mut`.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised, then holds 5.
    let r = unsafe {
        b.write(0, 5);
        b.slice(0, 1)
    };
    b.grow(8);
    print(r.len());
    // SAFETY: slot 0 holds the 5; then every slot is uninitialised.
    unsafe {
        b.read(0);
        b.free();
    }
}
// error: cannot borrow `b` as mutable while `b` is borrowed
