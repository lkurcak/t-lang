// There's one quotient for each sum and divisor: `b / 2` isn't `a / 2`.
fn f(a: u64, b: u64) -> { r: u64 | r == a / 2 } { b / 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(4, 6))?;
    Ok(())
}
// error: can't prove `r == a / 2` for the result of `f`
