// Sub-slices proved in range, and the lengths `split_at`, `str::get` and
// their kin say (docs/56-field-paths-and-slices.md).

use alloc::list::List;

// A row of a grid stored row by row: `y * w` and `y * w + w` are in it.
fn row(
    cells: &[u8],
    w: usize,
    y: { y: usize | y * w + w <= cells.len() },
) -> { r: &[u8] | r.len() == w } { cells.slice(y * w..y * w + w) }

// The halves `split_at` gives say how long they are.
fn first_of_tail(s: &[u8], mid: usize) -> u8 {
    match s.split_at(mid) {
        Some(sp) => match sp.tail.len() {
            0 => 0,
            _ => sp.tail[0],
        },
        None => 0,
    }
}

// A branch's slices keep their lengths where branches meet.
fn either(s: &[u8], c: bool) -> u8 {
    if s.len() < 4 {
        return 0;
    }
    let t = if c { s.slice(0..2) } else { s.slice(2..4) };
    t[1]
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: [u8; 6] = [1, 2, 3, 4, 5, 6];
    let s = a.as_slice();
    let t = s.slice(1..4);
    let n: { n: usize | n == 3 } = t.len();
    out.line(&n)?;                   // expect: 3
    out.line(&t[2])?;                // expect: 4
    out.line(&row(s, 3, 1)[2])?;     // expect: 6
    out.line(&first_of_tail(s, 4))?; // expect: 5
    out.line(&first_of_tail(s, 6))?; // expect: 0
    out.line(&either(s, true))?;     // expect: 2
    out.line(&either(s, false))?;    // expect: 4
    // An array's and a list's, too, and to change.
    let u = a.slice_mut(4..6);
    u[1] = 9;
    out.line(&a[5])?; // expect: 9
    let mut l: List<u8> = List::new();
    l.push(7);
    l.push(8);
    let v: { v: usize | v == 1 } = l.slice(1..2).len();
    out.line(&v)?; // expect: 1
    l.slice_mut(0..1)[0] = 6;
    // The borrow `slice_mut` gives back outlives the call, so what's known
    // of `l` is forgotten: `get` it.
    if let Some(x) = l.get(0) {
        out.line(x)?; // expect: 6
    }
    // A string literal's length is known; `get` says how long what it
    // gives is, and `split_at` its halves.
    let word = "hello";
    let five: { f: usize | f == 5 } = word.len();
    out.line(&five)?; // expect: 5
    if let Some(m) = word.get(1, 4) {
        let k: { k: usize | k == 3 } = m.len();
        out.line(&k)?; // expect: 3
        out.line(&m)?; // expect: ell
    }
    if let Some(sp) = word.split_at(2) {
        let h: { h: usize | h == 2 } = sp.head.len();
        let r: { r: usize | r == 3 } = sp.tail.len();
        out.line(&(h + r))?; // expect: 5
    }
    // What `trim` and `strip_prefix` give is no longer.
    let padded = "  hi ";
    let trimmed = padded.trim();
    let shorter: { s: bool | s } = trimmed.len() <= 5;
    out.line(&shorter)?; // expect: true
    if let Some(rest) = word.strip_prefix("he") {
        let at_most: { m: bool | m } = rest.len() <= 5;
        out.line(&at_most)?; // expect: true
    }
    Ok(())
}
