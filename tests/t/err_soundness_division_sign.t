// Division rounds towards zero, so for a negative `a`, `a / b * b` is
// *more* than `a`: -7 / 2 is -3, and -3 * 2 is -6.
fn floor(a: { a: i64 | a > -100 && a < 100 }, b: { b: i64 | b > 0 && b < 100 }) -> {
    r: i64 | r <= a
} {
    a / b * b
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&floor(-7, 2))?;
    Ok(())
}
// error: can't prove `r <= a` for the result of `floor`
