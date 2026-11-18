// An `==` of whole values inside an `||` says one of the two: the caller
// can't count on either (docs/85-values-in-predicates.md).
use alloc::list::List;
use core::mem::swap;

fn maybe(a: &mut List<u8>, b: &mut List<u8>, go: bool) ensures *a == old(*b) || *a == old(*a) {
    if go {
        swap(a, b);
    }
}

fn main() with alloc {
    let mut a: List<u8> = List::new();
    let mut b: List<u8> = List::new();
    b.push(1);
    maybe(&mut a, &mut b, true);
    let _ = a[0];
}
// error: can't prove `i < self.len()`
