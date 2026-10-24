#[derive(Discard)]
enum C { R, G }
fn f(c: C) -> i64 {
    match c {
        C::R | C::R => 1,
        C::G => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(C::R))?;
    out.line(&f(C::G))?;
    Result::Ok(())
}
// warning: unreachable alternative
// expect: 1
// expect: 2
