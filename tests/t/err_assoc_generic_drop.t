// Generic code can't throw away a `T::Out` the trait doesn't say is
// `Discard`.
trait Tr {
    type Out;

    fn out(&self) -> Self::Out;
}

fn f<T: Tr>(t: &T) -> int {
    let _x = t.out();
    1
}

fn main() {}
// error: `_x` is never used up, and its type, `T::Out`, isn't `Discard`; to let it be thrown away, add a bound: `T: Tr<Out: Discard>`
