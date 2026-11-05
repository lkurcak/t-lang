fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let o: Option<i64> = None;
    out.line(&o.unwrap())?;
    Ok(())
}
// error: runtime error: panicked
