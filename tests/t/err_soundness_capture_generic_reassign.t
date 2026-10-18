// The same, with the captured variable given a new value after: the closure
// has a copy of the old one, which would still be fine.
fn call<T: Copy>(x: T, y: T) -> T {
    let mut v = x;
    let g = || v;
    v = y;
    let _w = v;
    g()
}

fn main() {
    let b = Box::new(5);
    let c = Box::new(6);
    print(*call(&*b, &*c));
}
// error: cannot use `call` with `T` = `&int` here: in that copy of it, this closure can't capture `v`
