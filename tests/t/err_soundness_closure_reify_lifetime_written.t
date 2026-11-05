// The same with the closure's result written, and the closure itself where
// the function is wanted.
fn keep(f: fn(&i64) -> &'static i64) -> &'static i64 {
    let n = 1;
    f(&n)
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = keep(|x: &i64| -> &i64 { x });
    out.line(&(*r))?;
    Ok(())
}
// error: this closure returns a reference from `x`, whose lifetime each call picks, but its result doesn't have that lifetime
