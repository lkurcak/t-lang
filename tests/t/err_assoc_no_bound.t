trait Tr {
    type Out;
}

fn f<T>(x: T::Out) -> T::Out { x }

fn main() {}
// error: none of `T`'s traits has an associated type `Out`
