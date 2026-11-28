// A bound that says `with {}` says calling the closure can't panic, so
// `apply` needn't say it may. This closure may. (One that says no `with`
// leaves what its calls do open: docs/104-open-fn-bounds.md.)
fn apply<F: Fn(i64) -> i64 with {} + Discard>(f: F) -> i64 { f(1) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let z = 0;
    out.line(&apply(|x| x / z))?;
    Ok(())
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn(i64) -> i64`, which can't panic
