// The recursion only shows through which impl a method call on a
// projection runs: `go::<T>` calls `show` on a `T::Out`, which is `W<T>`
// here, and `W<U>`'s `show` calls `go::<W<U>>`: `go` at a bigger type each
// time round. A call on `T::Out` may run any impl, at a type as much
// bigger as an impl likes.
#[derive(Discard)]
struct W<T> { t: T }

trait Show {
    fn show(self, n: i64) -> i64 with panic;
}

trait Tr {
    type Out: Show;

    fn wrap(self) -> Self::Out;
}

impl<T: Discard> Tr for W<T> {
    type Out = W<W<T>>;

    fn wrap(self) -> W<W<T>> { W { t: self } }
}

impl<U: Discard> Show for W<U> {
    fn show(self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { go(self, n - 1) }
    }
}

fn go<T: Tr>(x: T, n: i64) -> i64 with panic { x.wrap().show(n) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&go(W { t: 1i64 }, 3))?;
    Result::Ok(())
}
// error: polymorphic recursion
