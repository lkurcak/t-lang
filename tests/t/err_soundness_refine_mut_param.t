// A `mut` parameter can change, so its refinement is an invariant, checked
// at each assignment.
fn f(mut x: { v: i64 | v > 0 }) -> i64 {
    x = 0;
    10 / x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(1))?;
    Result::Ok(())
}
// error: can't prove `v > 0` for `x`
