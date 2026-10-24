// An inherent impl's parameters may be decided by a bound too.
#[derive(Discard where F: Discard)]
struct Thunk<F> { f: F }

impl<U, F: Fn() -> U> Thunk<F> {
    fn call(&self) -> U { (self.f)() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k: i64 = 5;
    let t = Thunk { f: || k };
    out.line(&t.call())?;    // expect: 5
    Result::Ok(())
}
