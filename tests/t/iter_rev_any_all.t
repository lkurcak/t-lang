// `rev` gives an iterator's values back to front, of one that can give
// them from the back (`DoubleEndedIterator`): a range, a slice's, and what
// `map`, `copied` and `rev` make of those. `any` and `all` stop at the
// first value that decides them, leaving the rest to iterate.
use alloc::list::List;

const KNIGHT: [(i8, i8); 3] = [(1, 2), (2, 1), (-1, 2)];

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    for k in (0..4u8).rev() {
        out.put(&k)?;
    }
    out.line("")?; // expect: 3210
    let mut l = List::new();
    l.push(1i64);
    l.push(2);
    l.push(3);
    for x in l.iter().rev() {
        out.put(&x)?;
    }
    out.line("")?; // expect: 321
    for x in l.iter().copied().rev().map(|x: i64| x * 10) {
        out.put(&x)?;
    }
    out.line("")?; // expect: 302010
    for x in l.iter().rev().rev() {
        out.put(&x)?;
    }
    out.line("")?; // expect: 123
    for (dx, dy) in KNIGHT.iter().copied() {
        out.put(&(dx + dy))?;
    }
    out.line("")?; // expect: 331
    let mut r = 0..10u8;
    out.line(&r.next_back().unwrap_or(99))?; // expect: 9
    out.line(&r.next().unwrap_or(99))?; // expect: 0
    out.line(&r.rev().next().unwrap_or(99))?; // expect: 8
    for x in l.iter_mut().rev() {
        *x = *x + 1;
    }
    out.line(&l[2])?; // expect: 4
    let mut it = l.iter().copied();
    out.line(&it.any(|x: i64| x == 3))?; // expect: true
    out.line(&it.next().unwrap_or(0))?; // expect: 4
    out.line(&it.any(|x: i64| x == 3))?; // expect: false
    out.line(&l.iter().all(|x: &i64| *x > 1))?; // expect: true
    out.line(&l.iter().all(|x: &i64| *x > 2))?; // expect: false
    out.line(&KNIGHT.iter().any(|d: &(i8, i8)| d.0 < 0))?; // expect: true
    out.line(&(0..0u8).all(|x: u8| x > 5))?; // expect: true
    Ok(())
}
