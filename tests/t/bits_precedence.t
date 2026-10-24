// A chain of one operator needs no parentheses, nor does a comparison of
// their results; mixing two of `&`, `|`, `^`, the shifts and arithmetic
// does.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: u8 = 1;
    let b: u8 = 2;
    let c: u8 = 4;
    out.line(&(a | b | c))?;               // expect: 7
    out.line(&(a & b == 0))?;         // expect: true
    out.line(&((a | b) & c))?;             // expect: 0
    out.line(&(a | (b & c)))?;             // expect: 1
    out.line(&((a + b) << 2))?;            // expect: 12
    let x: u16 = 0x1234;
    out.line(&((x >> 8) as u8 & 0x0F))?;   // expect: 2
    Result::Ok(())
}
