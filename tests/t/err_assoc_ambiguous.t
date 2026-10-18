// Two of `T`'s traits have an `Out`: say which.
trait A {
    type Out;
}

trait B {
    type Out;
}

fn f<T: A + B>(x: T::Out) -> T::Out { x }

fn main() {}
// error: `T::Out` could be `<T as A>::Out` or `<T as B>::Out`
