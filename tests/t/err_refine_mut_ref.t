// Through a `&mut i64`, anything could be written to `i`, which would break
// its refinement: lent straight to a call, it's proved again after, and
// `zero` doesn't say what it leaves.
fn zero(r: &mut i64) { *r = 0; }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut i: { v: i64 | v > 0 } = 1;
    zero(&mut i);
    out.line(&i)?;
    Ok(())
}
// error: can't prove `v > 0` for `i` after the call to `zero`
