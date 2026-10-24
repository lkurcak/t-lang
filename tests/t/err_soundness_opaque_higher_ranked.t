// `Fn(&i64) -> &i64` returns a reference as long-lived as its argument,
// whatever that is, but this closure returns `x`, which lives for `'a`.
fn keep<'a>(x: &'a i64) -> impl Fn(&i64) -> &i64 + Discard { move |_y| x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 1;
    let f = keep(&n);
    let m = 2;
    out.line(&(*f(&m)))?;
    Result::Ok(())
}
// error: this closure's result has the lifetime of `_y`, which each call picks
