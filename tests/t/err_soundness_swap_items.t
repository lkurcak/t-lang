// After `swap`, each range is the other's: `x` gives what `y` did, values
// from 90 on, not below 8 (docs/84-counting.md, docs/82-item-invariants.md).
use core::mem::swap;

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut x = 0..8;
    let mut y = 90..100;
    swap(&mut x, &mut y);
    for r in x {
        let _ = a[r];
    }
    let _ = y;
}
// error: can't prove `i < self.len()`
