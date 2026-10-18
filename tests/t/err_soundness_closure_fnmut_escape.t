// A `FnMut` closure can't hand out a `&mut` to what it captured: two calls
// would give two `&mut x` at once.
fn main() {
    let mut x = 0;
    let mut c = || &mut x;
    let a = c();
    let b = c();
    *a = 1;
    *b = 2;
    print(x);
}
// error: this closure returns a reference to what it captured, which only lasts as long as the call
