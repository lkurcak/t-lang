// `a <= 5`, but `b` may be negative, so `a * b <= 5 * b` doesn't follow:
// `-1 * -1` is 1, and `5 * -1` is -5.
fn f(
    a: { a: i64 | a > -10 && a <= 5 },
    b: { b: i64 | b > -10 && b < 10 },
) -> { p: i64 | p <= 5 * b } { a * b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, -1))?;
    Ok(())
}
// error: can't prove `p <= 5 * b` for the result of `f`
