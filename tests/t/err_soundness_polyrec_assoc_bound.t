// Through what an associated type's declared bound picks: `go` calls
// `show` on a `T::Out`, whose impl is the one `W<U>`'s impl of `Tr` proved
// `V<U>: Show` by, as `type Out: Show` says it must. So `V<U>`'s `show`
// calls `go` at `W<W<U>>`, whose `Out` is `V<W<U>>`, whose `show` calls `go`
// at `W<W<W<U>>>`, and so on: the edge is from the impl of `Tr`, which
// picked `V`'s impl of `Show` (see docs/29-closed-and-sealed-traits.md).
trait Show {
    fn show(&self, n: i64) -> i64 with panic;
}

trait Tr {
    type Out: Show + Discard;

    fn out(&self) -> Self::Out;
}

fn go<T: Tr>(x: &T, n: i64) -> i64 with panic { x.out().show(n) }

#[derive(Copy, Discard)]
struct W<U> { u: U }

#[derive(Copy, Discard)]
struct V<U> { u: U }

impl<U: Copy> Tr for W<U> {
    type Out = V<U>;

    fn out(&self) -> V<U> { V { u: self.u } }
}

impl<U: Copy> Show for V<U> {
    fn show(&self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { go(&W { u: W { u: self.u } }, n - 1) }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&go(&W { u: 1i64 }, 3))?;
    Result::Ok(())
}
// error: polymorphic recursion
