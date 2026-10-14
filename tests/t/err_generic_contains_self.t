// `A` holds a `Box<A>`, which holds an `A`.
struct Box<T> { x: T }
struct A { b: Box<A> }
fn main() {}
// error: `A` contains itself
