fn f(n: i64) -> i64 {
    match n {
        0..=9 | 3 => 1,
        _ => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(3))?;
    out.line(&f(10))?;
    Result::Ok(())
}
// warning: unreachable
// expect: 1
// expect: 2
