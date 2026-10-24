fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"\q")?;
    Result::Ok(())
}
// error: unknown escape `\q`
