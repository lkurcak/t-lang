// An impl's method of a trait has its trait's method's effects, and what
// that says of where it has them: it can't say `unless` itself
// (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

trait Bump {
    fn bump(&self) -> u8 with panic;
}

impl Bump for A {
    fn bump(&self) -> u8 with panic unless A: Small { 1 }
}

fn main() {}
// error: an impl's method of a trait can't say `unless`: it has what its trait's method says, where that says it
