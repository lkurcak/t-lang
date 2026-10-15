// `A` holds a `Wrap<A>`, which holds an `A`.
struct Wrap<T> { x: T }
struct A { b: Wrap<A> }
fn main() {}
// error: `A` contains itself
