#[derive(Discard)]
struct W<T> { x: T }
impl<T> W<T> { fn get(&self) {} }
impl W<i64> { fn get(&self) {} }
fn main() {}
// error: method `get` is defined for `W<i64>` here, and for `W<T>` in another impl, and those can be the same type
