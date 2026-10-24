#[derive(Discard)]
struct P { x: i64 }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { x: 1 };
    out.line(&p.z)?;
    Result::Ok(())
}
// error: `P` has no field `z`
