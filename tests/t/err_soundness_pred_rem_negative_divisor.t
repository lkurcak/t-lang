// By a negative constant, `%` still has the sign of what's divided, and
// `/` still rounds towards zero: 7 % -2 is 1, and 7 / -2 is -3, so this
// claim, of floored division's -1, is false.
fn f(a: { v: i64 | v == 7 }) -> { r: i64 | r / -2 == -3 && r % -2 == -1 } { a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(7))?;
    Ok(())
}
// error: can't prove `r / -2 == -3 && r % -2 == -1` for the result of `f`
