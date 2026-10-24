use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let r = &*Box::new(1i64);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: a temporary value is dropped here while still borrowed
