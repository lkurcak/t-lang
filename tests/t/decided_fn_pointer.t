// What a bound decides may be a function type generic over lifetimes of
// its own: those mean something in it, wherever it is.
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

fn id(x: &i64) -> &i64 { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = Thunk { f: || id };
    let g = t.get();
    let n = 5;
    out.line(&(*g(&n)))?;  // expect: 5
    Result::Ok(())
}
