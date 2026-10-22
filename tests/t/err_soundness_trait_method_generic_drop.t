// An impl's method is checked at its own type parameters, like any generic
// function: it can't throw a `T` away.
struct Fd { n: i64 }

trait Sink {
    fn put<T>(&self, x: T) -> i64;
}

#[derive(Discard)]
struct S {}

impl Sink for S {
    fn put<T>(&self, _x: T) -> i64 { 0 }
}

fn main() {
    print(S {}.put(Fd { n: 1 }));
}
// error: `_x` is never used up, and its type, `T`, isn't `Discard`; to let it be thrown away, add a bound: `T: Discard`
