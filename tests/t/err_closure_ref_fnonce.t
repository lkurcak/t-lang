// What can be called only once can't be through a reference: the call
// takes it.
fn call<F: FnOnce() -> int>(f: &F) -> int { f() }

fn main() {}
// error: `&F` can't be called; that needs `F: Fn`
