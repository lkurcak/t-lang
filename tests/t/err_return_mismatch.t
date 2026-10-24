fn f(c: bool) -> i64 {
    if c { return true; }
    1
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(true))?;
    Result::Ok(())
}
// error: expected `i64`, found `bool`
