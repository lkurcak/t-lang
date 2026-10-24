fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let o: Option<i64> = Option::None;
    out.line(&o.unwrap())?;
    Result::Ok(())
}
// error: runtime error: panicked
