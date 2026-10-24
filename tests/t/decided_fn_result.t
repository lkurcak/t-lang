// An impl's type parameter needn't be in its type if a bound decides it:
// here `U` is what `F` gives, which `F`, part of `Thunk<F>`, says.
trait Get {
    type Out;

    fn get(&self) -> Self::Out;
}

#[derive(Discard where F: Discard)]
struct Thunk<F> { f: F }

impl<U, F: Fn() -> U> Get for Thunk<F> {
    type Out = U;

    fn get(&self) -> U { (self.f)() }
}

fn twice<G: Get<Out = i64>>(g: &G) -> i64 with panic { g.get() + g.get() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let t = Thunk { f: || 21 };
    out.line(&twice(&t))?;                   // expect: 42
    let b = Thunk { f: || Option::Some(7i64) };
    match b.get() {
        Option::Some(n) => out.line(&n)?,    // expect: 7
        Option::None => {},
    }
    Result::Ok(())
}
