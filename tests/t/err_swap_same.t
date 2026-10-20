// A place can't be swapped with itself: that's two `&mut`s of it at once.

use core::mem::swap;

fn main() {
    let mut x = 1;
    swap(&mut x, &mut x);
    print(x);
}
// error: cannot borrow `x` as mutable more than once at a time
