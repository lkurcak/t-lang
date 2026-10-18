// Even with one impl that fits, `T` isn't worked out from `T::Out`: which
// impls there are mustn't change what a call means.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = int;
}

fn mk<T: Tr>(x: T::Out) -> T::Out { x }

fn main() {
    print(mk(1));
}
// error: cannot infer
