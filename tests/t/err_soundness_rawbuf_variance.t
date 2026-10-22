// A buffer of `&'a mut` is no buffer of `&'static mut`: it would let a
// short-lived reference be written where a long-lived one is read.
use alloc::raw::RawBuf;

fn keep<'a>(b: &mut RawBuf<&'a int>, r: &'a int) with unsafe { b.write(0, r) }

fn main() with {alloc, panic} {
    let mut b: RawBuf<&'static int> = RawBuf::new(1);
    {
        let n = 5;
        // SAFETY: slot 0 is uninitialised.
        unsafe { keep(&mut b, &n) };
    }
    // SAFETY: slot 0 holds a reference; then every slot is uninitialised.
    unsafe {
        print(*b.read(0));
        b.free();
    }
}
// error: `n` doesn't live long enough
