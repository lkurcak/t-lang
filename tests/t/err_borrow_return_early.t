// A reference returned early has to live as long as the result's lifetime
// says, as one returned at the end does.
fn pick<'a>(a: &'a i64, c: bool) -> &'a i64 {
    let local = 5;
    if c { return &local; }
    a
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 1;
    out.line(&(*pick(&x, false)))?;
    Result::Ok(())
}
// error: `local` doesn't live long enough: it goes away when the function returns
