// Covered by the arm above, alternative by alternative.
#[derive(Discard)]
enum C { R, G, B }
fn f(c: C) -> i64 {
    match c {
        C::R | C::G => 1,
        C::B => 2,
        C::G | C::B => 3,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(C::R))?;
    out.line(&f(C::G))?;
    out.line(&f(C::B))?;
    Result::Ok(())
}
// warning: unreachable pattern
// expect: 1
// expect: 1
// expect: 2
