// A generic function's `T::Out` has the lifetimes of the `T` it's called
// with: `keep(&x)` gives back a reference to `x`.
trait Tr {
    type Out;

    fn out(self) -> Self::Out;
}

impl<'a> Tr for &'a i64 {
    type Out = &'a i64;

    fn out(self) -> &'a i64 { self }
}

fn keep<T: Tr>(t: T) -> T::Out { t.out() }

fn main() {
    let r;
    {
        let x: i64 = 1;
        r = keep(&x);
    }
    print(*r);
}
// error: `x` doesn't live long enough
