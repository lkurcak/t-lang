// Not a top-level alternative, so checking those alone would miss it.
#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o {
        Opt::Some(1 | 1) => 1,
        _ => 0,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Opt::Some(1)))?;
    out.line(&f(Opt::Some(2)))?;
    out.line(&f(Opt::None))?;
    Result::Ok(())
}
// warning: unreachable alternative
// expect: 1
// expect: 0
// expect: 0
