// Earlier alternatives count, as for unreachable alternatives.
fn f(n: i64) -> i64 {
    match n {
        0..10 | 5..20 => 1,
        _ => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(3))?;
    out.line(&f(15))?;
    out.line(&f(20))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `10..20`
// expect: 1
// expect: 1
// expect: 2
