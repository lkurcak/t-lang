// `r` itself, and a reborrow of what it points to.

use alloc::boxed::Box;
use core::mem::swap;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut x = Box::new(1i64);
    let r = &mut x;
    swap(r, &mut *r);
    out.line(&(*x))?;
    Result::Ok(())
}
// error: cannot borrow `*r` as mutable more than once at a time
