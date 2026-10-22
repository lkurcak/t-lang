// A closure's effects are its body's, and an `unsafe` block around the
// closure doesn't take on what calling it later does: the obligations are
// kept, or not, where it's called.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: the closure isn't called here.
    let mut f = unsafe { |b: &mut RawBuf<int>| b.read(0) };
    print(f(&mut b));
    // SAFETY: the slot is uninitialised.
    unsafe { b.free() };
}
// error: `f` is unsafe, but it isn't in an `unsafe` block
// warning: this `unsafe` block has nothing unsafe in it
