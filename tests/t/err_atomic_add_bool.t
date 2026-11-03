// Adding is for integers.
use core::sync::Atomic;

fn main() {
    let a = Atomic::new(true);
    let _ = a.fetch_add(true);
// error: `Atomic<T>::fetch_add` needs its `T` to implement `Int`, and `bool` doesn't
}
