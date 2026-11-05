// A hole names what goes in it: there are no arguments after the string.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"x = {}")?;
    Result::Ok(())
}
// error: a hole in a format string names what goes in it, as in `{x}`
