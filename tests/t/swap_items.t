// After `swap`, each range is the other's, and gives what it did: `y` is
// `0..8`, so what it gives indexes an array of 8 (docs/84-counting.md).
use core::mem::swap;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut x = 6..8;
    let mut y = 90..100;
    swap(&mut x, &mut y);
    for r in y {
        out.line(&a[r])?;
    }
    // expect: 16
    // expect: 17
    let _ = x;
    Ok(())
}
