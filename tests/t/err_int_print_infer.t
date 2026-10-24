// `print` takes any integer type, so it doesn't decide a literal's.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&5)?;
    Result::Ok(())
}
// error: cannot infer
