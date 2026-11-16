// What a range gives through an adapter: each value of `a..b` is at least
// `a` and less than `b`, whatever its adapters do (docs/82-item-invariants.md).
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    for r in (5..8).rev() {
        out.line(&a[r])?;
    }
    // expect: 17
    // expect: 16
    // expect: 15
    for r in (6..8).rev().into_iter() {
        out.line(&a[r])?;
    }
    // expect: 17
    // expect: 16
    for r in (3..8).skip(3) {
        out.line(&a[r])?;
    }
    // expect: 16
    // expect: 17
    for (i, r) in (6..8).rev().enumerate() {
        out.line(&f"{i} {a[r]}")?;
    }
    // expect: 0 17
    // expect: 1 16
    for (r, q) in (0..8).zip((0..8).rev()).skip(6) {
        out.line(&f"{a[r]} {a[q]}")?;
    }
    // expect: 16 11
    // expect: 17 10
    for r in (0..8).filter(|x| *x % 3 == 0) {
        out.line(&a[r])?;
    }
    // expect: 10
    // expect: 13
    // expect: 16
    // `a..b`'s own lower bound, and from the back.
    let b: [u8; 3] = [1, 2, 3];
    for r in 5..8 {
        out.line(&b[r - 5])?;
    }
    // expect: 1
    // expect: 2
    // expect: 3
    let mut it = 2..5;
    while let Some(r) = it.next_back() {
        out.line(&b[r - 2])?;
    }
    // expect: 3
    // expect: 2
    // expect: 1
    // Outside a loop: each call keeps it.
    let mut rv = (0..8).rev().skip(6);
    let x = rv.next();
    let y = rv.next();
    let z = rv.next();
    match (x, y, z) {
        (Some(i), Some(j), None) => out.line(&f"{a[i]} {a[j]}"), // expect: 11 10
        _ => Ok(()),
    }
}
