// A bound without `unsafe` doesn't hold of an unsafe closure: `twice`
// calls `f` with no block of its own.
use alloc::raw::RawBuf;

fn twice<F: FnMut() -> int + Discard>(mut f: F) -> int with panic { f() + f() }

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: slot 0 is the only one, and it's read once.
    print(unsafe { twice(|| b.read(0)) });
    // SAFETY: the slot is uninitialised.
    unsafe { b.free() };
}
// error: this closure is unsafe, but `twice` needs calls of it to fit `fn() -> int`, which can't be unsafe
