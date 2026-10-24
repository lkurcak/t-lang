fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: i64 = 3;
    let b: { v: i64 | v > a } = 3;
    out.line(&b)?;
    Result::Ok(())
}
// error: can't prove `v > a` for `b`
