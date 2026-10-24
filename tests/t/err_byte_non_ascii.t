// A byte literal holds bytes, written as ASCII or `\x..`: `é` is two.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&b"café".len())?;
    Result::Ok(())
}
// error: a byte literal holds only ASCII characters; write other bytes as `\x..`
