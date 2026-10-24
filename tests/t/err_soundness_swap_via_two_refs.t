// `r` itself, and a reborrow of what it points to.

use alloc::boxed::Box;
use core::mem::swap;

fn main() with alloc {
    let mut x = Box::new(1i64);
    let r = &mut x;
    swap(r, &mut *r);
    print(*x);
}
// error: cannot borrow `*r` as mutable more than once at a time
