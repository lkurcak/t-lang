// Lending what's in an `Unsafe` as `&` is unsafe too: an `as_mut` loan
// could change it while the `&` lives.
use core::cell::Unsafe;

fn main() {
    let c = Unsafe::new(1u32);
    let a = c.as_ref();
    // error: unsafe
    let _ = *a;
}
