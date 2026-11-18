// An argument that borrows `b`, and keeps it borrowed, while the call
// borrows it again, through `r`, given `&mut b` by that argument. Since
// step 83 that `r` can't be given a new value there at all (docs/83).

use alloc::boxed::Box;
use core::mem::swap;

fn main() with {alloc, panic} {
    let mut a = Box::new(1i64);
    let mut b = Box::new(2);
    let mut r = &mut a;
    swap(&mut *r, { r = &mut b; &mut b });
    let _v = *a + *b;
}
// error: can't give `r` a new value here: an earlier argument borrows `*r`
