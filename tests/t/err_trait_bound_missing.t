// Nothing is known about a `T` but what its bounds say.
trait Show { fn show(&self); }
fn f<T>(x: T) { x.show(); }
fn main() {}
// error: `T` has no method `show`: its bounds don't give it one; `Show` has one, so a bound `T: Show` would
