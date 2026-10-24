// `-x` of a `u8` panics unless it's 0.
fn neg(x: u8) -> u8 with panic { -x }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&neg(0))?;  // expect: 0
    out.line(&neg(1))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
