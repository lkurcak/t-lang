// Changing what a `&` points to is as wrong in a closure as anywhere.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    let r = &n;
    let mut set = || *r = 1;
    set();
    out.line(&n)?;
    Result::Ok(())
}
// error: cannot assign to `*r`: it's behind a `&` reference
