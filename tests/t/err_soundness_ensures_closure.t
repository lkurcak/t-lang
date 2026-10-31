// A closure that captures what's lent may change it whenever it's called:
// `ensures` is about the value at return, which is then gone.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

fn set(c: &mut Counter) ensures c.get() == 5 {
    c.n = 5;
    let mut f = || c.n = 0;
    f();
}

fn main() {}
// error: can't prove `c.get() == 5` for what `set` ensures
