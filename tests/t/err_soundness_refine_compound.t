// `+=` is `+`, and must fit too.
fn f(x: { v: i64 | v >= 0 }) -> i64 with {} {
    let mut y = x;
    y += 1;
    y
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(9223372036854775807))?;
    Result::Ok(())
}
// error: `+=` may panic, but `f` doesn't say `with panic`
