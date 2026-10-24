// A bound without `unsafe` doesn't hold of an unsafe closure: `twice`
// calls `f` with no block of its own.
use alloc::raw::RawBuf;

fn twice<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 with panic { f() + f() }

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    // SAFETY: slot 0 is the only one, and it's read once.
    let _v = unsafe { twice(|| b.read(0)) };
    // SAFETY: the slot is uninitialised.
    unsafe { b.free() };
}
// error: this closure is unsafe, but `twice` needs calls of it to fit `fn() -> i64`, which can't be unsafe
