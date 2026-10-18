// `S<int>` holds an `<int as Tr>::Out`, which is `S<int>`: a type that
// holds itself, of infinite size, though neither `S` nor the impl says so
// alone.
trait Tr {
    type Out;
}

struct S<T: Tr> { x: T::Out }

impl Tr for int {
    type Out = S<int>;
}

fn main() {}
// error: contains itself
