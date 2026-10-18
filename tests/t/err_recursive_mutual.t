#[derive(Discard)]
struct A { b: B }
#[derive(Discard)]
struct B { a: A }
fn main() {}
// error: `A` contains itself
