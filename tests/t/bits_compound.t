fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut x: u8 = 0b1111_0000;
    x &= 0b1010_1010;
    out.line(&x)?;   // expect: 160
    x |= 1;
    out.line(&x)?;   // expect: 161
    x ^= 0xFF;
    out.line(&x)?;   // expect: 94
    x >>= 2;
    out.line(&x)?;   // expect: 23
    x <<= 3;
    out.line(&x)?;   // expect: 184
    Result::Ok(())
}
