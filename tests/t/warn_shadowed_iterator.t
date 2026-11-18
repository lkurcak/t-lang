// A `for` whose pattern binds the name it gives its iterator: in the body
// `x` is the value, in the invariant the iterator, which is easy to misread
// (docs/85-values-in-predicates.md).
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let xs: [u8; 2] = [1, 2];
    for x in x: xs.iter() invariant x.len() <= 2 {
        out.line(x)?;
    }
    Ok(())
}
// warning: the `for`'s pattern binds `x`, which hides the iterator `x` in its body
// expect: 1
// expect: 2
