// A new `d` is a new variable: what was known of the old one isn't of it.
fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let d: i64 = 1;
    let d = d - 1;
    out.line(&(10 / d))?;
    Result::Ok(())
}
// error: `/` may panic, but `main` doesn't say `with panic`
