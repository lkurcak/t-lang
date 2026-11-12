// Escapes: each means one thing. `\u{..}` is a character, written as
// UTF-8; `\x..` is a byte, any in byte literals, and only ASCII, a
// character of its own, in strings and characters.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("tab:\tend")?; // expect: tab:	end
    out.line("quote: \" and \\ and \'")?; // expect: quote: " and \ and '
    out.line(&"\u{20AC}".len())?; // expect: 3
    out.line(&"€".len())?; // expect: 3
    out.line(&"\0".len())?; // expect: 1
    out.line(&b"\xFF\x00".len())?; // expect: 2
    out.line(&b'\xFF')?; // expect: 255
    out.line(&b'a')?; // expect: 97
    out.line(&b'\n')?; // expect: 10
    out.line(&('\u{1F600}' as u32))?; // expect: 128512
    out.line(&('\'' as u32))?; // expect: 39
    out.line("\x41\x7e")?; // expect: A~
    out.line(&"\x00\x7F".len())?; // expect: 2
    out.line(&('\x08' as u32))?; // expect: 8
    let n = 1u8;
    out.line(&f"\x41{n}")?; // expect: A1
    out.line(&"two
lines".len())?; // expect: 9
    Ok(())
}
