// A reference is the same value wherever it points, so `*a == old(*b)`
// holds of two `&mut`s swapped, but what they point to may change after:
// the caller learns which reference `ra` is, not what's behind it
// (docs/85-values-in-predicates.md).
use core::mem::swap;

fn change<'x>(a: &mut &'x mut u8, b: &mut &'x mut u8) ensures *a == old(*b) {
    swap(a, b);
    **a = 9;
}

fn main() {
    let mut x: u8 = 1;
    let mut y: u8 = 2;
    let mut ra = &mut x;
    let mut rb = &mut y;
    change(&mut ra, &mut rb);
    let _two: { v: u8 | v == 2 } = *ra;
}
// error: can't prove `v == 2`
