fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f: impl Fn() -> i64 = || 1;
    out.line(&f())?;
    Result::Ok(())
}
// error: `impl` types can only be written in a function's return type
