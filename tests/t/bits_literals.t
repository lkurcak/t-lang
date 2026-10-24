// Hexadecimal, binary and octal literals, with `_` between digits and a
// suffix, as decimal ones.
fn class(b: u8) -> u8 {
    match b {
        0x00..=0x7F => 1,
        0x80..=0xBF => 0,
        0xC0..=0xFF => 2,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&0xFFu8)?;              // expect: 255
    out.line(&0b1010_0000u8)?;       // expect: 160
    out.line(&0o17u8)?;              // expect: 15
    out.line(&0x7FFF_FFFFi32)?;      // expect: 2147483647
    let m: i8 = -0x80;
    out.line(&m)?;                   // expect: -128
    out.line(&class(0x41))?;         // expect: 1
    out.line(&class(0xA9))?;         // expect: 0
    out.line(&class(0xE2))?;         // expect: 2
    Result::Ok(())
}
