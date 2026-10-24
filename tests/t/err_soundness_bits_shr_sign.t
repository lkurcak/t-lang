// `x >> n` keeps the sign of `x`.
fn f(a: i64, n: u32) -> { r: i64 | r >= 0 } { a >> n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-8, 1))?;
    Result::Ok(())
}
// error: can't prove `r >= 0` for the result of `f`
