// The closure changes `d`, when it's called.
fn f() -> i64 {
    let mut d = 1;
    let mut c = || d = 0;
    c();
    10 / d
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f())?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
