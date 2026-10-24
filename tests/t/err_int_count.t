// `count` gives a `usize`, which is whatever the iterator counted.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let r: core::ops::Range<i8> = -128..127;
    out.line(&r.count())?;   // expect: 255
    let none: u8 = 5;
    out.line(&(none - 6))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
