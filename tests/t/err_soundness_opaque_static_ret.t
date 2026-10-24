// The closure returns `x`, a `&'a i64`, which isn't a `&'static i64`.
fn pick<'a>(x: &'a i64) -> impl Fn() -> &'static i64 + Discard { move || x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 1;
    out.line(&(*pick(&n)()))?;
    Result::Ok(())
}
// error: lifetime may not live long enough
