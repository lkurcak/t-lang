fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = 1;
    x = 2;
    out.line(&x)?;
    Result::Ok(())
}
// error: cannot assign to `x`, as `x` isn't declared `mut`
