// `S<T>` holds a box of a `T::Out`, and `<W<U> as Tr>::Out` is an
// `S<W<W<U>>>`: so `S<W<int>>` holds an `S<W<W<int>>>`, which holds an
// `S<W<W<W<int>>>>`, and so on. Each is finite, through a box, but there's
// no end of them to monomorphise.
#[derive(Discard)]
struct W<T> { t: T }

trait Tr {
    type Out;
}

struct S<T: Tr> { b: Box<T::Out> }

impl<U> Tr for W<U> {
    type Out = S<W<W<U>>>;
}

fn size(_s: &S<W<int>>) -> int { 0 }

fn main() {
    print(1);
}
// error: polymorphic recursion
