// Calling an `FnMut` closure borrows it mutably, which it can't be while
// `r` does.
fn main() with panic {
    let mut n: i64 = 0;
    let mut c = || n = n + 1;
    let r = &mut c;
    c();
    r();
}
// error: cannot borrow `c` as mutable more than once at a time
