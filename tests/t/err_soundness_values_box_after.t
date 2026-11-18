// What a box owns is part of its value: after the swap, `*a` is the box
// `*b` was, but writing what it owns makes it another value
// (docs/85-values-in-predicates.md).
use alloc::boxed::Box;
use core::mem::swap;

fn change(a: &mut Box<u8>, b: &mut Box<u8>) ensures *a == old(*b) {
    swap(a, b);
    **a = 5;
}

fn main() with alloc {
    let mut a = Box::new(1);
    let mut b = Box::new(2);
    change(&mut a, &mut b);
}
// error: can't prove `*a == old(*b)` for what `change` ensures
