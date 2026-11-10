// An iterator adapter gives what its iterator gives: `rev`, `skip`,
// `enumerate`, `zip` and `copied` keep what a type argument says of the
// iterator's values, through the adapter's associated `Item` (docs/63).
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut l: List<{ t: usize | t < 8 }> = List::new();
    l.push(7);
    l.push(0);
    l.push(3);
    for i in l.iter().rev() {
        out.line(&a[*i])?;
    }
    // expect: 13
    // expect: 10
    // expect: 17
    for i in l.iter().skip(1) {
        out.line(&a[*i])?;
    }
    // expect: 10
    // expect: 13
    for (k, i) in l.iter().enumerate() {
        out.line(&f"{k} {a[*i]}")?;
    }
    // expect: 0 17
    // expect: 1 10
    // expect: 2 13
    for (i, j) in l.iter().zip(l.iter().rev()) {
        out.line(&f"{a[*i]} {a[*j]}")?;
    }
    // expect: 17 13
    // expect: 10 10
    // expect: 13 17
    for i in l.as_slice().iter().copied().rev() {
        out.line(&a[i])?;
    }
    // expect: 13
    // expect: 10
    // expect: 17
    Ok(())
}
