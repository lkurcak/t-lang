// An `unsafe` block in a closure's body keeps that body's obligations, so
// the closure isn't unsafe to call. One that doesn't is, and its caller
// keeps them, in a block, or by being unsafe itself.
use alloc::raw::RawBuf;

fn apply<e, F: FnOnce() -> int with e>(f: F) -> int with e { f() }

/// The caller must know slot 0 holds a value, and leave it uninitialised.
fn take_first(b: &mut RawBuf<int>) -> int with unsafe { apply(|| b.read(0)) }

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    let put = |b: &mut RawBuf<int>, first: bool, v: int| {
        let i = if first { 0 } else { 1 };
        // SAFETY: wrong in general, but each slot is written once here, and
        // there are two.
        unsafe { b.write(i, v) }
    };
    put(&mut b, true, 3);
    put(&mut b, false, 4);
    // SAFETY: slot 1 holds the 4, and `take` isn't called again.
    let take = || unsafe { b.read(1) };
    print(apply(take)); // expect: 4
    // SAFETY: slot 0 holds the 3; then every slot is uninitialised.
    unsafe {
        print(take_first(&mut b)); // expect: 3
        // SAFETY: the inner block keeps `free`'s obligation, so it's the
        // outer block's no longer.
        unsafe { b.free() };
    }
}
