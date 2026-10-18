// Two impls for the same type whose associated types are each worked out
// from the other's: neither is about a smaller type.
#[derive(Discard)]
struct W<T> { t: T }

trait A {
    type X;
}

trait B {
    type Y;
}

impl<T> A for W<T> {
    type X = <W<T> as B>::Y;
}

impl<T> B for W<T> {
    type Y = <W<T> as A>::X;
}

fn main() {}
// error: smaller
