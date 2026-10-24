// `\x` is for bytes; a character is `\u{..}`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"\x41")?;
    Result::Ok(())
}
// error: `\x` escapes are for byte literals; write a character as `\u{..}`
