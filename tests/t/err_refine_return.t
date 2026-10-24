// The result must have the refinement the signature promises.
fn pos(x: i64) -> { r: i64 | r > 0 } { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pos(1))?;
    Result::Ok(())
}
// error: can't prove `r > 0` for the result of `pos`
