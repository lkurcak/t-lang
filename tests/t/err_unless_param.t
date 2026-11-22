// An effect parameter is what's given: only `panic`, `alloc` and `global`
// can say `unless` (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f<effect e, T>(_x: &T) with e unless T: Small {}

fn main() {}
// error: only `panic`, `alloc` and `global` can say `unless`
