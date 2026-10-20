// `f::<T>` uses `f::<T::Out>`, and `T::Out` may be a bigger type than `T`
// (here `W<T>`), each time round: monomorphising would never end.
#[derive(Discard)]
struct W<T> { t: T }

trait Tr {
    type Out: Tr;

    fn next(self) -> Self::Out;

    fn done(self) -> int;
}

impl<T: Discard> Tr for W<T> {
    type Out = W<W<T>>;

    fn next(self) -> W<W<T>> { W { t: self } }

    fn done(self) -> int { 0 }
}

fn f<T: Tr>(x: T, n: int) -> int with panic {
    if n == 0 { x.done() } else { f(x.next(), n - 1) }
}

fn main() with panic {
    print(f(W { t: 1 }, 3));
}
// error: polymorphic recursion
