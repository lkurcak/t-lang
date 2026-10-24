fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x: i64 = 1;
    let r = &x;
    x = 2;
    out.line(&(*r))?;
    Result::Ok(())
}
// error: cannot assign to `x` while `x` is borrowed
