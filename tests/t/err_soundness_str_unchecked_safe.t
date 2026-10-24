// Making a `str` of bytes nobody checked is unsafe: they might not be
// UTF-8.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = b"\xFF";
    let s = core::str::from_utf8_unchecked(b);
    out.line(&s)?;
    Result::Ok(())
}
// error: `core::str::from_utf8_unchecked` is unsafe, but it isn't in an `unsafe` block
