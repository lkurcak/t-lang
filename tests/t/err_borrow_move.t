use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let b = Box::new(1i64);
    let r = &b;
    let c = b;
    out.line(&(**r))?;
    out.line(&(*c))?;
    Result::Ok(())
}
// error: cannot move out of `b` while `b` is borrowed
