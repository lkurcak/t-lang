// An operator panics if its result doesn't fit in its type.
fn add(a: u8, b: u8) -> u8 with panic { a + b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&add(200, 56))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
