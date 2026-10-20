// Giving `b` a new value frees the old box, which `r` points into.
fn main() with {alloc, panic} {
    let mut b = Box::new(1);
    let r = &*b;
    b = Box::new(2);
    print(*r);
}
// error: cannot assign to `b` while `*b` is borrowed
