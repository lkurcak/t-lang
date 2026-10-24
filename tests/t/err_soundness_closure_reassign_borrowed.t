// `g` borrows `v`, so `v` can't be given a new value while `g` is still to
// be used: what `g` reads would change under it. (`move || v` would take a
// copy instead.)

use alloc::boxed::Box;

fn call<T: Copy>(x: T, y: T) -> T {
    let mut v = x;
    let g = || v;
    v = y;
    let _w = v;
    g()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(5);
    let c = Box::new(6);
    out.line(&(*call(&*b, &*c)))?;
    Result::Ok(())
}
// error: cannot assign to `v` while `v` is borrowed
