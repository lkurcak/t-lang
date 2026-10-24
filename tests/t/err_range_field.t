fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = 0..3;
    out.line(&r.start)?;
    Result::Ok(())
}
// error: field `start` of `Range` is private
