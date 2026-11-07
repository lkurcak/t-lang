// What's in an `Unsafe` is private to `core::cell`: reaching it is only by
// `as_mut` and `as_ref`, which are unsafe.
use core::cell::Unsafe;

fn main() {
    let c = Unsafe::new(1u32);
    let r = &c.value;
    // error: private
    let _ = *r;
}
