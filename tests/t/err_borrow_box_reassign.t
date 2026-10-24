// Giving `b` a new value frees the old box, which `r` points into.

use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Box::new(1i64);
    let r = &*b;
    b = Box::new(2);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: cannot assign to `b` while `*b` is borrowed
