// A variable declared with no value, and not `mut`, gets one value.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: i64;
    r = 1;
    r = 2;
    out.line(&r)?;
    Result::Ok(())
}
// error: cannot assign twice to `r`, as it isn't declared `mut`: it may have a value already
