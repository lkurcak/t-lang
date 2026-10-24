// A variable bound by an or-pattern is one variable, with one warning, and
// the fix renames it in every alternative.
#[derive(Discard)]
enum Pair { L(i64, i64), R(i64, i64) }
fn f(p: Pair) -> i64 {
    match p {
        Pair::L(a, b) | Pair::R(b, a) => a,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Pair::R(1, 2)))?;
    out.line(&f(Pair::L(3, 4)))?;
    Result::Ok(())
}
// warning: unused variable `b`
// expect: 2
// expect: 3
