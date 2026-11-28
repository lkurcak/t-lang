// A function that isn't exported and says its effects can say more, or
// drop its `with` and have its body's.
use alloc::boxed::Box;

fn helper<effect e, F: Fn() with e + Discard>(f: F) with e {
    f();
    let _b = Box::new(1u8);
}

fn main() { helper(|| {}) }
// error: `Box::new` may allocate, but `helper` doesn't say `with alloc`
