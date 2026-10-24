// An `unsafe` block in a closure's body keeps that body's obligations, so
// the closure isn't unsafe to call. One that doesn't is, and its caller
// keeps them, in a block, or by being unsafe itself.
use alloc::raw::RawBuf;

fn apply<effect e, F: FnOnce() -> i64 with e>(f: F) -> i64 with e { f() }

/// The caller must know slot 0 holds a value, and leave it uninitialised.
fn take_first(b: &mut RawBuf<i64>) -> i64 with unsafe { apply(|| b.read(0)) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b: RawBuf<i64> = RawBuf::new(2);
    let put = |b: &mut RawBuf<i64>, first: bool, v: i64| {
        let i = if first { 0 } else { 1 };
        // SAFETY: wrong in general, but each slot is written once here, and
        // there are two.
        unsafe { b.write(i, v) }
    };
    put(&mut b, true, 3);
    put(&mut b, false, 4);
    // SAFETY: slot 1 holds the 4, and `take` isn't called again.
    let take = || unsafe { b.read(1) };
    let four = apply(take);
    // SAFETY: slot 0 holds the 3; then every slot is uninitialised.
    let three = unsafe {
        let three = take_first(&mut b);
        // SAFETY: the inner block keeps `free`'s obligation, so it's the
        // outer block's no longer.
        unsafe { b.free() };
        three
    };
    out.line(&four)?; // expect: 4
    out.line(&three) // expect: 3
}
