// `S<i64>` holds an `<i64 as Tr>::Out`, which is `S<i64>`: a type that
// holds itself, of infinite size, though neither `S` nor the impl says so
// alone.
trait Tr {
    type Out;
}

struct S<T: Tr> { x: T::Out }

impl Tr for i64 {
    type Out = S<i64>;
}

fn main() {}
// error: contains itself
