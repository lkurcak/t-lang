// Every warning is reported, not just the first, in source order.
fn f(n: i64) -> i64 {
    match n {
        0..=5 => 1,
        5..=10 => 2,
        3 => 3,
        _ => 4,
    }
}
fn g(b: bool) -> i64 {
    match b {
        true | true => 1,
        false => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    out.line(&f(6))?;
    out.line(&g(true))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `6..=10`
// warning: unreachable pattern
// warning: unreachable alternative
// expect: 1
// expect: 2
// expect: 1
