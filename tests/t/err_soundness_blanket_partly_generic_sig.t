// A generic function's signature can't name the blanket impl's associated
// type of `Wrap<T>` unless its bound holds: at `Wrap<Foo>` it's another
// impl's.
trait B {}

trait Tr {
    type A;
}

impl<X: B> Tr for X {
    type A = i64;
}

#[derive(Discard)]
struct Wrap<T> { t: T }

#[derive(Discard)]
struct Foo {}

impl Tr for Wrap<Foo> {
    type A = bool;
}

fn f<T>(n: <Wrap<T> as Tr>::A) -> <Wrap<T> as Tr>::A { n } // error: `Wrap<T>` doesn't implement `Tr`

fn main() {}
