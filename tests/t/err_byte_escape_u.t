// `\u{..}` is a character: in a byte literal, write the byte.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&b"\u{41}".len())?;
    Result::Ok(())
}
// error: `\u{..}` escapes are for characters; write a byte as `\x..`
