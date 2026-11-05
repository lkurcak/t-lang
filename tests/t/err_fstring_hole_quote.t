// A hole can't hold a `"`: anything that reads the format string without
// knowing holes would see it end there.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"{"x".len()}")?;
    Result::Ok(())
}
// error: a hole in a format string can't hold a `"`; work out what needs one in a `let` first
