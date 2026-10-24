// An assignment inside a closure must keep the refinement too.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut i: { v: i64 | v > 0 } = 1;
    let mut c = || i = 0;
    c();
    out.line(&i)?;
    Result::Ok(())
}
// error: can't prove `v > 0` for `i`
