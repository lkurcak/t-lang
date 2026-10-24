// `swap` makes both places hold a reference that lives as long as the
// other did: `a` can't end up pointing into `short`.

use alloc::boxed::Box;
use core::mem::swap;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let long = Box::new(1i64);
    let mut a = &*long;
    {
        let short = Box::new(2);
        let mut b = &*short;
        swap(&mut a, &mut b);
        out.line(&(*b))?;
    }
    out.line(&(*a))?;
    Result::Ok(())
}
// error: `short` doesn't live long enough
