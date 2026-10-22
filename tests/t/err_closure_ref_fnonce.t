// What can be called only once can't be through a reference: the call
// takes it.
fn call<F: FnOnce() -> i64>(f: &F) -> i64 { f() }

fn main() {}
// error: `&F` can't be called; that needs `F: Fn`
