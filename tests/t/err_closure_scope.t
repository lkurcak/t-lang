// A closure's parameters are only in scope in its body.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: i64| x;
    out.line(&(f(1) + x))?;
    Result::Ok(())
}
// error: unknown variable `x`
