// `as` says which number didn't fit, a negative one too.
fn byte(n: i64) -> u8 with panic { n as u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&byte(0))?; // expect: 0
    out.line(&byte(-3))?;
    Result::Ok(())
}
// error: runtime error: `as` overflowed: -3 isn't a `u8`
