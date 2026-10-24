// Through a `&mut i64`, anything could be written to `i`, which would break
// its refinement.
fn zero(r: &mut i64) { *r = 0; }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut i: { v: i64 | v > 0 } = 1;
    zero(&mut i);
    out.line(&i)?;
    Result::Ok(())
}
// error: can't borrow `i` as `&mut`: its type has a refinement, which a write through the reference could break
