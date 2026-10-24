// A `let mut` keeps its refinement: each assignment must have it.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut i: { v: i64 | v >= 0 } = 0;
    i = i - 1;
    out.line(&i)?;
    Result::Ok(())
}
// error: can't prove `v >= 0` for `i`
