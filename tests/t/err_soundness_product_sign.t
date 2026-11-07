// `b >= 0` but nothing's known of `a`'s sign, so `a * b` can be negative:
// `-1 * 1` is.
fn f(a: { a: i64 | a > -10 && a < 10 }, b: { b: i64 | b >= 0 && b < 10 }) -> { p: i64 | p >= 0 } {
    a * b
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, 1))?;
    Ok(())
}
// error: can't prove `p >= 0` for the result of `f`
