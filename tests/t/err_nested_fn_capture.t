// A nested `fn` can't use the locals around it: it means what it would at
// the top level. The fix makes it a closure, which can.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 2;
    /// Adds `n`.
    fn add_n(x: i64) -> i64 { x + n }
    out.line(&add_n(1))?;
    Result::Ok(())
}
// error: can't capture `n` in a nested `fn`; use a closure
