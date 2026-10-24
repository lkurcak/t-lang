fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = { let inner = 1; inner };
    out.line(&inner)?;
    Result::Ok(())
}
// error: unknown variable `inner`
