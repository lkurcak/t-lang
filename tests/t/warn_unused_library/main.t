// A library listed but never named is one the package doesn't need: a
// target without it could still run it.
// warning: the `alloc` library is in `libraries`, but nothing in this package names it
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1_i64)?; // expect: 1
    Result::Ok(())
}
