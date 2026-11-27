// A float isn't `Ord`: a NaN is neither less nor greater than anything, so
// there's no total order of them by `<`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&core::cmp::max(1.5f64, 2.5))?;
    Ok(())
}
// error: `core::cmp::max` needs its `T` to implement `Ord`, and `f64` doesn't
