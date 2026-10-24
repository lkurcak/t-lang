// A closure holding a `&mut` isn't `Copy`: a copy would be a second `&mut`
// to the same `n`.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    let mut a = || n = n + 1;
    let mut b = a;
    a();
    b();
    out.line(&n)?;
    Result::Ok(())
}
// error: use of moved value `a`
