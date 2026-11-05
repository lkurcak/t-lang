// What a hole names is looked up as any variable is, where the string is.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"{nope}")?;
    Result::Ok(())
}
// error: unknown variable `nope`
