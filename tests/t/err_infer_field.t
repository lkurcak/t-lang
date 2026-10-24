// Reading a field needs the type now: which struct is it?
#[derive(Discard)]
struct P { x: i64 }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let get = |p| p.x;
    out.line(&get(P { x: 1 }))?;
    Result::Ok(())
}
// error: the type of this must be known to read its field `x`
