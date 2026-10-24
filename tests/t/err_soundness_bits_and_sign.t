// `a & b` is negative when both are: -1 & -1 is -1.
fn f(a: i8, b: i8) -> { r: i8 | r >= 0 } { a & b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, -1))?;
    Result::Ok(())
}
// error: can't prove `r >= 0` for the result of `f`
