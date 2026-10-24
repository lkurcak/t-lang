// The other side of empty_enum.t: an arm for a variant that can't hold a
// value can never run.
#[derive(Discard)]
enum Never {}
#[derive(Discard)]
enum Maybe { Real(i64), Impossible(Never) }
fn get(m: Maybe) -> i64 {
    match m {
        Maybe::Real(x) => x,
        Maybe::Impossible(n) => match n {},
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&get(Maybe::Real(4)))?;
    Result::Ok(())
}
// warning: variant `Maybe::Impossible` is never constructed
// warning: unreachable pattern
// expect: 4
