trait Show { fn show(&self); }
fn f<T: Show + Discard>(_x: T) {}
fn main() { f(1i64); }
// error: `f` needs its `T` to implement `Show`, and `i64` doesn't
