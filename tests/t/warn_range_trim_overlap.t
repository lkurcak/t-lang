// The first arm already matches 10..20, so the second only ever gets 20..30.
fn f(n: i64) -> i64 {
    match n {
        0..20 => 1,
        10..30 => 2,
        _ => 3,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(15))?;
    out.line(&f(25))?;
    out.line(&f(30))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `20..30`
// expect: 1
// expect: 2
// expect: 3
