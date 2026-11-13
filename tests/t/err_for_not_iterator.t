fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for x in 3i64 {
        out.line(&x)?;
    }
    Ok(())
}
// error: `i64` can't be iterated: it isn't an iterator, nor `IntoIterator`
