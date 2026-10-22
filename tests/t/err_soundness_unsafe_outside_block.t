// An unsafe operation's obligation is its caller's to keep, so it's only
// allowed where something says it's kept: an `unsafe` block, or a
// function that says `with unsafe`, passing it on to its callers.
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    b.write(0, 1);
    print(b.read(0));
    b.free();
}
// error: `write` is unsafe, but it isn't in an `unsafe` block
