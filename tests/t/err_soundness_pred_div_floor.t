// A predicate's `/` rounds towards zero, as a body's does: -7 / 2 is -3.
// Rounding down would make it -4, which this claims.
fn f(a: { v: i64 | v == -7 }) -> { r: i64 | r / 2 == -4 } { a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-7))?;
    Ok(())
}
// error: can't prove `r / 2 == -4` for the result of `f`
