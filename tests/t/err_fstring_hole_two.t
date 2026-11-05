// A hole holds one expression.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = 1i64;
    let b = 2i64;
    out.line(&f"{a b}")?;
    Result::Ok(())
}
// error: a hole in a format string holds one expression; this is after it
