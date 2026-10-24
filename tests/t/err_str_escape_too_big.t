// Nothing is past `10FFFF`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"\u{110000}")?;
    Result::Ok(())
}
// error: `\u{110000}` isn't a Unicode scalar value
