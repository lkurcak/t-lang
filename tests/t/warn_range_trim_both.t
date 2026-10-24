// Both ends covered: narrowed on both sides.
fn f(n: i64) -> i64 {
    match n {
        0 => 1,
        10 => 2,
        0..=10 => 3,
        _ => 4,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(0))?;
    out.line(&f(5))?;
    out.line(&f(10))?;
    out.line(&f(11))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `1..=9`
// expect: 1
// expect: 3
// expect: 2
// expect: 4
