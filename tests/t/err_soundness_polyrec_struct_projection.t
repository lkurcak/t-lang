// Through a struct's projection, which the impl picked where the struct is
// named works out: `S<W<U>>`'s field is a `<W<U> as Tr>::Out`, which is
// `S<W<W<U>>>`, whose field is a `<W<W<U>>> as Tr>::Out`, and so on, so
// monomorphising `S<W<i64>>` makes types without end. The edge is from the
// impl's `Out`, which names `S<W<W<U>>>` and so proves `W<W<U>>: Tr`, by
// the impl itself (see docs/29-closed-and-sealed-traits.md).
use alloc::boxed::Box;

trait Tr {
    type Out;
}

struct S<T: Tr> { b: Option<Box<T::Out>> }

struct W<U> { u: U }

impl<U> Tr for W<U> {
    type Out = S<W<W<U>>>;
}

fn main() {
    let _s: S<W<i64>> = S { b: Option::None };
}
// error: polymorphic recursion
