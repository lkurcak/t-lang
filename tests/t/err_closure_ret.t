fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: i64| -> bool { x + 1 };
    out.line(&f(1))?;
    Result::Ok(())
}
// error: expected `bool`, found `i64`
