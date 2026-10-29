// Only one branch makes `d` non-zero.
fn f(c: bool) -> i64 with {} {
    let mut d = 0;
    if c {
        d = 1;
    }
    10 / d
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(false))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
