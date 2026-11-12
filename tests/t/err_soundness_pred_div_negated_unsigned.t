// `-x` of a `u64` is never positive, so only the law's half for what isn't
// positive is needed: -3 / 2 is -1, rounding towards zero. The other half
// would round it down, to -2, which this claims.
fn f(x: { v: u64 | v == 3 }) -> { r: bool | -x / 2 == -2 } { true }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(3))?;
    Ok(())
}
// error: can't prove `-x / 2 == -2` for the result of `f`
