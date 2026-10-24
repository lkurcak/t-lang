fn apply(f: fn(i64) -> i64) -> i64 { f(1) }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&apply(|x: bool| 1))?;
    Result::Ok(())
}
// error: expected `i64`, found `bool`
