// A range whose start is after its end is empty; written with constants,
// it's most likely a countdown that won't run.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for i in 5..0i64 {
        out.line(&i)?;
    }
    out.line(&1i64)?; // expect: 1
    Result::Ok(())
}
// warning: this range is empty: `5` is after `0`, so it never yields anything
