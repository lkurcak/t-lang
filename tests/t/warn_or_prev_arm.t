// The `1` is covered by the arm above. The arm as a whole is still
// reachable (through `2` and `None`), so this is only visible per alternative.
#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o {
        Opt::Some(1) => 1,
        Opt::Some(1 | 2) | Opt::None => 2,
        _ => 0,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Opt::Some(1)))?;
    out.line(&f(Opt::Some(2)))?;
    out.line(&f(Opt::None))?;
    out.line(&f(Opt::Some(3)))?;
    Result::Ok(())
}
// warning: unreachable alternative
// expect: 1
// expect: 2
// expect: 2
// expect: 0
