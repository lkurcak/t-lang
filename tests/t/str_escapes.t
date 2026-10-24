// Escapes: each means one thing. `\u{..}` is a character, written as
// UTF-8; `\x..` is a byte, only in byte literals.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"tab:\tend")?; // expect: tab:	end
    out.line(&"quote: \" and \\ and \'")?; // expect: quote: " and \ and '
    out.line(&"\u{20AC}".len())?; // expect: 3
    out.line(&"€".len())?; // expect: 3
    out.line(&"\0".len())?; // expect: 1
    out.line(&b"\xFF\x00".len())?; // expect: 2
    out.line(&b'\xFF')?; // expect: 255
    out.line(&b'a')?; // expect: 97
    out.line(&b'\n')?; // expect: 10
    out.line(&('\u{1F600}' as u32))?; // expect: 128512
    out.line(&('\'' as u32))?; // expect: 39
    out.line(&"two
lines".len())?; // expect: 9
    Result::Ok(())
}
