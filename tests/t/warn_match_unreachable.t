#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o {
        Opt::Some(x) => x,
        _ => 0,
        Opt::None => 1,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Opt::Some(7)))?;
    out.line(&f(Opt::None))?;
    Result::Ok(())
}
// warning: unreachable pattern
// expect: 7
// expect: 0
