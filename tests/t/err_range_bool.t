// A range of anything can be written, but only one of `i64`s is an
// iterator.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for b in false..true {
        out.line(&b)?;
    }
    Result::Ok(())
}
// error: `Range<bool>` doesn't implement `Iterator`
