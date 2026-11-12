// A bound on `I::e` is an upper bound: inside, `I::e` is still only
// itself, so a function that allocates isn't a `fn() with I::e`, which
// `0..3`'s would call without `alloc`.
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

fn leak<I: Iterator<e = {alloc}> + Discard>(_i: I) -> fn() with I::e { allocates }

fn main() {
    let f = leak(0u8..3);
    f();
}
// error: expected `fn() -> () with I::e`, found `fn() -> () with alloc`
