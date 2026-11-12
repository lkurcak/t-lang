// When the closure runs, `d` may not be what it was when it was made: a
// `move` closure copies `d`, 0, and is given to `apply` after `d` is set
// to 1. The closure's `d` is the copy, not what `d` is at the call. (One
// that borrows `d` runs only while `d` can't change: `frozen` in
// loan_forgets.t.)
fn apply<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut d = 0;
    let c = move || 10 / d;
    d = 1;
    out.line(&apply(c))?;
    out.line(&d)?;
    Ok(())
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn() -> i64`, which can't panic
