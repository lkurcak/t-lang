// After `swap`, the second place holds what the first did: `y` gives
// values from 90 on, as `x` did, not below 8 (docs/84-counting.md).
use core::mem::swap;

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut x = 90..100;
    let mut y = 0..8;
    swap(&mut x, &mut y);
    for r in y {
        let _ = a[r];
    }
    let _ = x;
}
// error: can't prove `i < self.len()`
