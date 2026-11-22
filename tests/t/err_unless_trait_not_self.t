// A trait's method's `unless` is of `Self`: what each impl's own method
// is checked against is its type (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

trait Bump {
    fn bump<T>(&self, _x: &T) -> u8 with panic unless T: Small { 1 }
}

fn main() {}
// error: a trait's method's `unless` is of `Self`, which each impl's type is
