// What's in an atomic is private to `core::sync`: changed only atomically.
use core::sync::Atomic;

fn main() {
    let a = Atomic::new(1u32);
    let _ = a.value;
// error: field `value` of `Atomic` is private to `core::sync`
}
