// A buffer of `&'a mut` is no buffer of `&'static mut`: it would let a
// short-lived reference be written where a long-lived one is read.
use alloc::raw::RawBuf;

/// Slot 0's value, moved out.
fn get_ref<'a>(b: &mut RawBuf<&'a i64>) -> &'a i64 with {panic, unsafe} {
    if b.cap() > 0 { b.read(0) } else { core::panic::panic("no slot") }
}

fn keep<'a>(b: &mut RawBuf<&'a i64>, r: &'a i64) with unsafe {
    if b.cap() > 0 { b.write(0, r) }
}

fn main() with {alloc, panic} {
    let mut b: RawBuf<&'static i64> = RawBuf::new(1);
    {
        let n = 5;
        // SAFETY: slot 0 is uninitialised.
        unsafe { keep(&mut b, &n) };
    }
    // SAFETY: slot 0 holds a reference; then every slot is uninitialised.
    unsafe {
        let _v = *get_ref(&mut b);
        b.free();
    }
}
// error: `n` doesn't live long enough
