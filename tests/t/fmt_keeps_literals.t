// `t fmt` keeps literals as they're written: escapes, and the case of
// their digits.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&('\u{10FFFF}' as u32))?; // expect: 1114111
    out.line(&"\u{41}\t\"")?;    // expect: A	"
    out.line(&b"\xFF\x0a".len())?;   // expect: 2
    out.line(&b'\x7F')?;             // expect: 127
    match 'é' {
        '\u{E9}' => out.line(&1u8)?, // expect: 1
        _ => out.line(&0u8)?,
    }
    Result::Ok(())
}
