fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: i64, x: i64| x;
    out.line(&f(1, 2))?;
    Result::Ok(())
}
// error: parameter `x` is declared twice
