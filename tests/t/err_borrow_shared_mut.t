fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut x: i64 = 1;
    let a = &x;
    let b = &mut x;
    *b += 1;
    out.line(&(*a))?;
    Result::Ok(())
}
// error: cannot borrow `x` as mutable while `x` is borrowed
