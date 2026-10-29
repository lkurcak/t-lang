// After `d = 0`, what was known of `d` before is about a value it no
// longer has.
fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let mut d: i64 = 1;
    d = 0;
    out.line(&(10 / d))?;
    Result::Ok(())
}
// error: `/` may panic, but `main` doesn't say `with panic`
