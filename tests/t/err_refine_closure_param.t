fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: { v: i64 | v > 0 }| x;
    out.line(&f(1))?;
    Ok(())
}
// error: a refinement can only be in the type of a function's parameter or result, a `let` or a field
