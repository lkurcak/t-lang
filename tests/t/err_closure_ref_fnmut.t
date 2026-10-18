// Calling through a `&F` calls `F` through a shared reference, which only
// works if it's `Fn`.
fn call<F: FnMut()>(f: &F) { f() }

fn main() {}
// error: `&F` can't be called; that needs `F: Fn`
