// Through what an opaque type's hidden type implements: `wrap` proves
// `V<U>: Show` for the `impl Show` it gives, so that's the impl `go` runs
// on one. `V<U>`'s `show` calls it on `wrap(W { .. })`, whose hidden type
// is `V<W<U>>`, whose `show` does the same at `V<W<W<U>>>`, and so on: the
// edge is from `wrap`, which picked the impl (see
// docs/29-closed-and-sealed-traits.md).
trait Show {
    fn show(&self, n: i64) -> i64 with panic;
}

fn go<T: Show>(x: &T, n: i64) -> i64 with panic { x.show(n) }

#[derive(Copy, Discard)]
struct W<U> { u: U }

#[derive(Copy, Discard)]
struct V<U> { u: U }

fn wrap<U: Copy>(u: U) -> impl Show + Copy { V { u } }

impl<U: Copy> Show for V<U> {
    fn show(&self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { go(&wrap(W { u: self.u }), n - 1) }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&go(&wrap(1i64), 3))?;
    Result::Ok(())
}
// error: polymorphic recursion
