// `enumerate` pairs each value with its index, from 0; `zip` pairs the
// values of two iterators, until either runs out.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let words = ["ab", "cde", "f"];
    for (i, w) in words.iter().enumerate() {
        out.line(&f"{i}: {w}")?;
        // expect: 0: ab
        // expect: 1: cde
        // expect: 2: f
    }
    let xs: [u8; 3] = [1, 2, 3];
    let ys: [u8; 4] = [10, 20, 30, 40];
    let mut dot: u32 = 0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        dot += (*x as u32) * (*y as u32);
    }
    out.line(&dot)?;                         // expect: 140
    let mut l: List<(usize, char)> = List::new();
    for p in "xyz".chars().enumerate() {
        l.push(p);
    }
    out.line(&f"{l:?}")?;                    // expect: [(0, 'x'), (1, 'y'), (2, 'z')]
    let n = xs.iter().zip(ys.iter()).zip("ab".chars()).count();
    out.line(&n)?;                           // expect: 2
    Ok(())
}
