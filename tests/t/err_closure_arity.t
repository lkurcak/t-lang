fn apply(f: fn(i64) -> i64) -> i64 { f(1) }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&apply(|x, y| x + y))?;
    Result::Ok(())
}
// error: expected `fn(i64) -> i64`, found `fn(_, _) -> _`
