// `\x` in a string or a character is ASCII, a character of its own: past
// `\x7F` it would be a byte of one, which isn't a character.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("caf\xE9")?;
    Ok(())
}
// error: `\xE9` isn't ASCII, so it's a byte, not a character; write the character as `\u{E9}`
