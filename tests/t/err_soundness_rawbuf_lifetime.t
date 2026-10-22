// A buffer of references holds them, so it can't outlive what they point
// to, even though its type has no fields saying so.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<&i64> = RawBuf::new(1);
    {
        let n = 5;
        // SAFETY: slot 0 is uninitialised.
        unsafe { b.write(0, &n) };
    }
    // SAFETY: slot 0 holds a reference.
    let r = unsafe { b.read(0) };
    print(*r);
    // SAFETY: every slot is uninitialised.
    unsafe { b.free() };
}
// error: `n` doesn't live long enough
