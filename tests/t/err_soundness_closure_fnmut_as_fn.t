// A closure that changes a captured variable is `FnMut`, not `Fn`: calling
// it through a shared reference could change `n` while someone reads it.
fn call<effect e, F: Fn() with e + Discard>(f: F) with e { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    call(|| n = n + 1);
    out.line(&n)?;
    Result::Ok(())
}
// error: this closure is only `FnMut`, as it changes `n`, but `call` needs it to implement `Fn`
