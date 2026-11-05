// A hole can't hold a comment.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 1i64;
    out.line(&f"{n // one
    }")?;
    Result::Ok(())
}
// error: a hole in a format string can't hold a comment
