// A place can't be swapped with itself: that's two `&mut`s of it at once.

use core::mem::swap;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x: i64 = 1;
    swap(&mut x, &mut x);
    out.line(&x)?;
    Result::Ok(())
}
// error: cannot borrow `x` as mutable more than once at a time
