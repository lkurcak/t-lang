fn f(a: bool) -> i64 {
    match a {
        _x => 1,
        true => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(true))?;
    Result::Ok(())
}
// warning: unreachable pattern
// expect: 1
