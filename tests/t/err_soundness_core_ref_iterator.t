// `&mut I` is an iterator, borrowing `I`; it can't outlive it.
fn keep<'a>(it: &'a mut core::ops::Range<i64>) -> &'a mut core::ops::Range<i64> { it }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r;
    {
        let mut it = 0..3;
        r = keep(&mut it);
    }
    out.line(&r.next().unwrap_or(0))?;
    Result::Ok(())
}
// error: `it` doesn't live long enough
