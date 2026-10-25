// Taking an array apart moves its values out of it.
use alloc::boxed::Box;

fn main() with alloc {
    let a = [Box::new(1i64), Box::new(2i64)];
    let [_x, _y] = a;
    let _z = a;
}
// error: use of moved value `a`
