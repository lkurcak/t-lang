// A single i64 at the end of a range counts too.
fn f(n: i64) -> i64 {
    match n {
        0..10 | 10 => 1,
        _ => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(9))?;
    out.line(&f(10))?;
    out.line(&f(11))?;
    Result::Ok(())
}
// warning: `0..10` and `10` can be merged into `0..=10`
// expect: 1
// expect: 1
// expect: 2
