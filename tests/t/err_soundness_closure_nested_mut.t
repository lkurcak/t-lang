// The inner closure changes `n`, so the outer one, which makes it, does
// too, through what it captured: it's `FnMut`, not `Fn`.
fn call<effect e, F: Fn() with e + Discard>(f: F) with e { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    call(|| {
        let mut g = || n = n + 1;
        g();
    });
    out.line(&n)?;
    Result::Ok(())
}
// error: this closure is only `FnMut`, as it changes `n`, but `call` needs it to implement `Fn`
