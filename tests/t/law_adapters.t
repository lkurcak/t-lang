// `map`, `rev` and `enumerate` of an exact iterator are as long as it, as
// laws of those methods say, and `next` of each takes one off: so a `for`
// over one counts with it (docs/88-iterator-laws.md).
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1u64, 2, 3, 4];
    let m = a.iter().map(|x: &u64| *x * 2);
    let k: { k: usize | k == 4 } = m.len();
    out.line(&k)?; // expect: 4
    let mut r = a.iter().rev();
    let first = r.next();
    let some: { b: bool | b } = first.is_some();
    out.line(&some)?; // expect: true
    let e = a.iter().enumerate();
    let n: { n: usize | n == 4 } = e.len();
    out.line(&n)?; // expect: 4
    let b = [10u64, 20, 30, 40];
    for x in it: a.iter().rev().map(|x: &u64| *x) invariant it.len() <= 4 {
        // `it` has one fewer than before `x` was given, so `3 - it.len()`
        // indexes `b`, from the front as `x` comes from the back.
        out.line(&(x + b[3 - it.len()]))?;
    }
    // expect: 14
    // expect: 23
    // expect: 32
    // expect: 41
    let z = a.iter().zip(b.iter());
    out.line(&z.len())?; // expect: 4
    Ok(())
}
