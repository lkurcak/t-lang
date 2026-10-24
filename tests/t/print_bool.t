fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&true)?;       // expect: true
    out.line(&(1i64 > 2))?;      // expect: false
    Result::Ok(())
}
