// Passing through a generic function forgets a refinement.
fn id<T>(x: T) -> T { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k: { v: i64 | v > 0 } = 5;
    let j: { v: i64 | v > 0 } = id(k);
    out.line(&j)?;
    Result::Ok(())
}
// error: can't prove `v > 0` for `j`
