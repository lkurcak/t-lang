// ...and it can't need more of `T` than the trait says, as callers go by
// the trait's signature.
struct Fd { n: int }

trait Sink {
    fn put<T>(&self, x: T) -> int;
}

#[derive(Discard)]
struct S {}

impl Sink for S {
    fn put<T: Discard>(&self, _x: T) -> int { 0 }
}

fn main() {
    print(S {}.put(Fd { n: 1 }));
}
// error: method `put` doesn't match its declaration in trait `Sink`: its bounds should be none
