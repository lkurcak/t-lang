fn f(x: u8) -> u8 with panic { x << 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(127))?;  // expect: 254
    out.line(&f(128))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
