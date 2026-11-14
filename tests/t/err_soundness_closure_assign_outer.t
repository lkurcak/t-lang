// A closure given to a variable from further out can't outlive what it
// borrows: here `*r.r`, which is in `n` (it captures through `r`'s
// reference since 0.91.0, docs/76-widths-and-captures.md).

use alloc::boxed::Box;

#[derive(Discard, Copy)]
struct R<'a> { r: &'a i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let f;
    {
        let n = Box::new(5);
        let r = R { r: &*n };
        f = || *r.r;
    }
    out.line(&f())?;
    Ok(())
}
// error: `n` doesn't live long enough
