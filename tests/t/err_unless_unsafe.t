// `unsafe` says what a caller must keep, not what the call does: no bound
// takes it away (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f<T>(_x: &T) with unsafe unless T: Small {}

fn main() {}
// error: only `panic`, `alloc` and `global` can say `unless`
