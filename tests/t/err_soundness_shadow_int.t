// A program's own `Int` doesn't give a type arithmetic: only core's sealed
// one does.
trait Int {}

#[derive(Discard, Copy, Eq)]
struct S {}

impl Int for S {}

fn add<T: Int + Copy>(a: T, b: T) -> T { a + b }

fn main() {}
// error: `+` works on integers, not on `T`; add a bound: `T: core::num::Int`
