// Inside, `F::e` is only itself: an allocating function isn't a `fn() with
// F::e`, which a caller whose closure does nothing would call as a `fn()`.
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

fn leak<F: Fn() + Discard>(_f: F) -> fn() with F::e { allocates }

fn main() {
    let f = leak(|| {});
    f();
}
// error: expected `fn() -> () with F::e`, found `fn() -> () with alloc`
