// Two ranges end to end are one range.
fn f(n: i64) -> i64 {
    match n {
        0..10 | 10..20 => 1,
        _ => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1))?;
    out.line(&f(0))?;
    out.line(&f(10))?;
    out.line(&f(19))?;
    out.line(&f(20))?;
    Result::Ok(())
}
// warning: `0..10` and `10..20` can be merged into `0..20`
// expect: 2
// expect: 1
// expect: 1
// expect: 1
// expect: 2
