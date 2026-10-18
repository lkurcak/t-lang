// `A` holds a `Wrap<A>`, which holds an `A`.
#[derive(Discard)]
struct Wrap<T> { x: T }
#[derive(Discard)]
struct A { b: Wrap<A> }
fn main() {}
// error: `A` contains itself
