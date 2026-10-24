// Two closures that both change `n` would each hold a `&mut n`.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    let mut a = || n = n + 1;
    let mut b = || n = n + 2;
    a();
    b();
    out.line(&n)?;
    Result::Ok(())
}
// error: cannot borrow `n` as mutable more than once at a time
