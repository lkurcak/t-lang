// Sharing one endpoint is almost always an off-by-one.
fn f(n: i64) -> i64 {
    match n {
        0..=5 => 1,
        5..=10 => 2,
        _ => 3,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    out.line(&f(6))?;
    out.line(&f(11))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `6..=10`
// expect: 1
// expect: 2
// expect: 3
