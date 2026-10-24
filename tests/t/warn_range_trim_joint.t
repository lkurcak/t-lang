// Two earlier arms cover the start between them.
fn f(n: i64) -> i64 {
    match n {
        0..10 => 1,
        10..20 => 2,
        5..30 => 3,
        _ => 4,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    out.line(&f(15))?;
    out.line(&f(25))?;
    out.line(&f(30))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `20..30`
// expect: 1
// expect: 2
// expect: 3
// expect: 4
