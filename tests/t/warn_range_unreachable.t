// Two ranges between them cover the third, though neither does alone.
fn f(n: i64) -> i64 {
    match n {
        0..10 => 1,
        10..=20 => 2,
        5..=15 => 3,
        _ => 4,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    out.line(&f(15))?;
    out.line(&f(21))?;
    Result::Ok(())
}
// warning: unreachable pattern
// expect: 1
// expect: 2
// expect: 4
