use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let b = Box::new(1i64);
    let c = b;
    out.line(&(*c))?;
    out.line(&(*b))?;
    Result::Ok(())
}
// error: use of moved value `b`
