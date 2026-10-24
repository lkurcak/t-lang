fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: { v: i64 | v > 0 }| x;
    out.line(&f(1))?;
    Result::Ok(())
}
// error: a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field
