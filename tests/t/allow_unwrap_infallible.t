// `#[allow(unwrap_infallible)]` turns the lint off.
fn sure(x: i64) -> Result<i64, !> { Result::Ok(x) }

#[allow(unwrap_infallible)]
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sure(1).unwrap_or(0))?; // expect: 1
    Result::Ok(())
}
