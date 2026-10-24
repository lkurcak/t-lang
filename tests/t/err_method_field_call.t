#[derive(Discard)]
struct F { f: fn() -> i64 }
fn one() -> i64 { 1 }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = F { f: one };
    out.line(&s.f())?;
    Result::Ok(())
}
// error: `F` has no method `f`, but it has a field `f`, which holds a function; to call that, put the field in parentheses
