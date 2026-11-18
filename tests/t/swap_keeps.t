// `swap` exchanges what its two places hold, so what was known of each is
// known of the other after: here, how long two lists are, and what's in a
// slice behind a reference (docs/84-counting.md).
use alloc::list::List;
use core::mem::swap;

fn longer(a: &mut List<u8>, b: &mut List<u8>) -> u8 with {} {
    if a.len() == 1 && b.len() == 0 {
        swap(a, b);
        return b[0];
    }
    0
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: List<u8> = List::new();
    a.push(7);
    let mut b: List<u8> = List::new();
    swap(&mut a, &mut b);
    out.line(&b[0])?;                   // expect: 7
    out.line(&longer(&mut b, &mut a))?; // expect: 7
    let mut x: { v: u8 | v < 8 } = 3;
    let mut y: { v: u8 | v < 8 } = 5;
    swap(&mut x, &mut y);
    let s: [u8; 6] = [0, 1, 2, 3, 4, 5];
    if x == 5 {
        out.line(&s[y as usize + 2])?; // expect: 5
    }
    Ok(())
}
