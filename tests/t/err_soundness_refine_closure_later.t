// When the closure runs, `d` may not be what it was when it was made.
fn apply<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut d = 1;
    let c = || 10 / d;
    out.line(&apply(c))?;
    d = 0;
    out.line(&d)?;
    Result::Ok(())
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn() -> i64`, which can't panic
