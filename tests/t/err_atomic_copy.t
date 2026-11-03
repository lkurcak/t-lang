// An atomic isn't `Copy`: a copy would be a second one, which a change to
// the first doesn't change.
use core::sync::Atomic;

fn main() {
    let a = Atomic::new(1u8);
    let b = a;
    let _ = a.load();
// error: use of moved value `a`
    let _ = b.load();
}
