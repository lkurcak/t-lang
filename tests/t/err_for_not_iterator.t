fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for x in 3i64 {
        out.line(&x)?;
    }
    Result::Ok(())
}
// error: `i64` doesn't implement `Iterator`
