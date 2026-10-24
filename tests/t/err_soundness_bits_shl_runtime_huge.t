// Far past the width: the interpreter mustn't wrap the amount.
fn f(x: u64, n: u32) -> u64 with panic { x << n }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(0, 4294967295))?;    // expect: 0
    out.line(&f(1, 4294967295))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
