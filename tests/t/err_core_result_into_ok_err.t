// `into_ok` is only for a `Result` that can't be an `Err`: one that can
// must say what to do with its error.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: Result<i64, bool> = Result::Ok(1);
    out.line(&r.into_ok())?;
    Result::Ok(())
}
// error: `Result<i64, bool>` has no method `into_ok`
