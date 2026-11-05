// A closure made a function is checked at the function type's lifetimes:
// `|x: &i64| x` returns what it's given, which isn't a `&'static i64`. If
// it were made a `fn(&i64) -> &'static i64`, `r` would point to `n` after
// it's gone.
fn keep(f: fn(&i64) -> &'static i64) -> &'static i64 {
    let n = 1;
    f(&n)
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let id = |x: &i64| x;
    let r = keep(id);
    out.line(&(*r))?;
    Ok(())
}
// error: this function isn't general enough
