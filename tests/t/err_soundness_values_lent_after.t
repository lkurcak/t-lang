// After the swap, `*a` is the list `*b` was, until it's lent `&mut` to
// `push` (docs/85-values-in-predicates.md).
use alloc::list::List;
use core::mem::swap;

fn change(a: &mut List<u8>, b: &mut List<u8>) with alloc ensures *a == old(*b) {
    swap(a, b);
    a.push(1);
}

fn main() with alloc {
    let mut a: List<u8> = List::new();
    let mut b: List<u8> = List::new();
    change(&mut a, &mut b);
}
// error: can't prove `*a == old(*b)` for what `change` ensures
