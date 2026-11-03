// Only integers and `bool` can be atomic: what the machine changes in one
// step.
use core::sync::Atomic;

fn main() {
    let a = Atomic::new('x');
// error: needs its `T` to implement `core::sync::Atom`, and `char` doesn't
    let _ = a.load();
}
