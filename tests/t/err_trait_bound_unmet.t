trait Show { fn show(&self); }
fn f<T: Show + Discard>(_x: T) {}
fn main() { f(1); }
// error: `f` needs its `T` to implement `Show`, and `int` doesn't
