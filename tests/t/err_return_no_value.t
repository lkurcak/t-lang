fn f(c: bool) -> i64 {
    if c { return; }
    1
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(true))?;
    Result::Ok(())
}
// error: a `return` with no value returns `()`, but this returns `i64`
