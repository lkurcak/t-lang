// Nor the locals of a function further out.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 1;
    fn outer() -> i64 {
        fn inner() -> i64 { n }
        inner()
    }
    out.line(&outer())?;
    Result::Ok(())
}
// error: can't capture `n` in a nested `fn`; use a closure
