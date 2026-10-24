// `r` only has a value if the `if` assigns one.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: i64;
    let c = true;
    if c {
        r = 1;
    }
    out.line(&r)?;
    Result::Ok(())
}
// error: use of `r`, which has no value here
