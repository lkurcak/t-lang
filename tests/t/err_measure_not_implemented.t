// `len` of a `map` of a `filter`: `Map` has one if what it maps does, and
// `Filter` doesn't, which is found once the closure is known
// (docs/88-iterator-laws.md).
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1u8, 2, 3];
    let m = a.iter().filter(|x: &&u8| **x > 1).map(|x: &u8| *x);
    for x in it: m invariant it.len() <= 3 {
        out.line(&x)?;
    }
    Ok(())
}
// error: doesn't implement `ExactSizeIterator`, whose measure this is
