// Nothing says which type `5` is, and there's no default.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 5;
    out.line(&(x == 5))?;
    Result::Ok(())
}
// error: cannot infer the type of this integer
