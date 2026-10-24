fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = loop {
        break;
    };
    out.line(&x)?;
    Result::Ok(())
}
// error: a `break` with no value gives the loop `()`, but it should be `i64`
