// An inherent impl's parameters may be decided by a bound too.
#[derive(Discard where F: Discard)]
struct Thunk<F> { f: F }

impl<U, F: Fn() -> U> Thunk<F> {
    fn call(&self) -> U { (self.f)() }
}

fn main() {
    let k = 5;
    let t = Thunk { f: || k };
    print(t.call());    // expect: 5
}
