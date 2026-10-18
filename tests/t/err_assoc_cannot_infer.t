// `mk`'s `T` is only in `T::Out`, and that `T::Out` is an `int` doesn't
// say which `T` it is: both impls give one. Working out a projection only
// goes forward, from the type to what it stands for.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = int;
}

impl Tr for () {
    type Out = int;
}

fn mk<T: Tr>(x: T::Out) -> T::Out { x }

fn main() {
    let x: int = mk(1);
    print(x);
}
// error: cannot infer
