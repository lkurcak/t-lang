// The same for `replace`.

use alloc::boxed::Box;
use core::mem::replace;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let long = Box::new(1i64);
    let mut a = &*long;
    {
        let short = Box::new(2);
        let old = replace(&mut a, &*short);
        out.line(&(*old))?;
    }
    out.line(&(*a))?;
    Result::Ok(())
}
// error: `short` doesn't live long enough
