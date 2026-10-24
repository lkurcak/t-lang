// Closures can't be compared, as functions can't.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 1;
    let a = || n;
    out.line(&(a == a))?;
    Result::Ok(())
}
// error: cannot compare closures with `==`
