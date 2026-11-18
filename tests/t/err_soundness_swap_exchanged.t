// After `swap`, each place holds what the other did: what was known of
// `a` is known of `b`, not of `a` (docs/84-counting.md).
use alloc::list::List;
use core::mem::swap;

fn main() with alloc {
    let mut a: List<u8> = List::new();
    a.push(7);
    let mut b: List<u8> = List::new();
    swap(&mut a, &mut b);
    let _ = a[0];
}
// error: can't prove `i < self.len()`
