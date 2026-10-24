// `&`, `|` and `^` work bit by bit, on two's complement: a negative
// number has infinitely many 1s on its left. Their result always fits in
// the type, so they never panic.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: u8 = 0b1100;
    let b: u8 = 0b1010;
    out.line(&(a & b))?;           // expect: 8
    out.line(&(a | b))?;           // expect: 14
    out.line(&(a ^ b))?;           // expect: 6
    let m: i8 = -1;
    let n: i8 = 5;
    out.line(&(m & n))?;           // expect: 5
    out.line(&(m | n))?;           // expect: -1
    out.line(&(m ^ n))?;           // expect: -6
    let lo: i8 = -128;
    out.line(&(lo & -1))?;         // expect: -128
    out.line(&(lo | 127))?;        // expect: -1
    let w: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    out.line(&(w ^ 0xFF))?;        // expect: 18446744073709551360
    out.line(&(0x0Fu32 & 0x3C))?;  // expect: 12
    Result::Ok(())
}
