// The bound is on a type parameter, which a call may or may not meet it
// with: of any other type it's already so or not
// (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f(_x: &A) with panic unless A: Small {}

fn main() {}
// error: a bound here can only be on a type parameter, `Self`, or an associated type of one, not `A`, which either meets it or doesn't
