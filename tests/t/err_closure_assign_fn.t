// A variable holding a function can be given a closure that captures
// nothing, but not one that captures.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut f: fn(i64) -> i64 = |x: i64| x;
    let n = 1;
    f = |x| x + n;
    out.line(&f(1))?;
    Result::Ok(())
}
// error: expected `fn(i64) -> i64`, found a closure that captures `n`
