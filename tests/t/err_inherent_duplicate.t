struct W<T> { x: T }
impl<T> W<T> { fn get(&self) {} }
impl W<int> { fn get(&self) {} }
fn main() {}
// error: method `get` is defined for `W<int>` here, and for `W<T>` in another impl, and those can be the same type
