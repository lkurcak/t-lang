// An `impl` result may hold every lifetime of the signature.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

fn later(c: &mut Counter) -> impl FnMut() ensures c.get() == 0 {
    c.n = 0;
    || c.n = 1
}

fn main() {}
// error: `ensures` can't name `c` at return: the borrow it's lent can outlive the call, as the lifetime of `c` is in the signature elsewhere; write `old(..)`
