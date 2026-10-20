// An argument that borrows `b`, and keeps it borrowed, while the call
// borrows it again.

use core::mem::swap;

fn main() {
    let mut a = Box::new(1);
    let mut b = Box::new(2);
    let mut r = &mut a;
    swap(&mut *r, { r = &mut b; &mut b });
    print(*a + *b);
}
// error: cannot borrow `b` as mutable more than once at a time
