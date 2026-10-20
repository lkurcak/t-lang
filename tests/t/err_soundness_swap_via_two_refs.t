// `r` itself, and a reborrow of what it points to.

use core::mem::swap;

fn main() {
    let mut x = Box::new(1);
    let r = &mut x;
    swap(r, &mut *r);
    print(*x);
}
// error: cannot borrow `*r` as mutable more than once at a time
