// Only values of the same type are compared whole
// (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
struct A { x: u8 }

#[derive(Copy, Discard)]
struct B { x: u8 }

fn f(a: &mut A, b: &B) ensures *a == *b {}

fn main() {}
// error: a refinement can't compare `*a`, of type `A`, with `*b`, of type `B`: only values of the same type
