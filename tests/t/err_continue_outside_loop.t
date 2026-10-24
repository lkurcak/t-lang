fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: i64| x + 1;
    out.line(&f(1))?;
    continue;
    Result::Ok(())
}
// error: `continue` outside of a loop
