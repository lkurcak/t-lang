// An `unsafe` block takes on the obligations of the operations in it,
// not of what a type parameter turns out to be: `call` can't know what
// `f`'s are. So `f`'s effects `e` stay `call`'s, and its caller, which
// passes an unsafe closure, must keep them.
use alloc::raw::RawBuf;

fn call<e, F: FnOnce() -> int with e>(f: F) -> int with e {
    // SAFETY: nothing here is unsafe but `f`, whose obligations aren't ours.
    unsafe { f() }
}

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    print(call(|| b.read(0)));
    // SAFETY: there's nothing in the slot to leave behind.
    unsafe { b.free() };
}
// error: `call` is unsafe, but it isn't in an `unsafe` block
// warning: this `unsafe` block has nothing unsafe in it
