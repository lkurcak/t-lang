// Changing what a `&` points to is as wrong in a closure as anywhere.
fn main() {
    let mut n: i64 = 0;
    let r = &n;
    let mut set = || *r = 1;
    set();
    print(n);
}
// error: cannot assign to `*r`: it's behind a `&` reference
