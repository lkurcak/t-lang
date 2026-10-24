fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = 1;
    out.line(&(*x))?;
    Result::Ok(())
}
// error: can't use `*` on `i64`: only on a reference or a box
