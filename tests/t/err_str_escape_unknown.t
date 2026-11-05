fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"\q")?;
    Ok(())
}
// error: unknown escape `\q`
