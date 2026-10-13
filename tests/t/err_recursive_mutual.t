struct A { b: B }
struct B { a: A }
fn main() {}
// error: `A` contains itself
