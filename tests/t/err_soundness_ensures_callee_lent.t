// What's lent to a call that says nothing is a value nothing's known of
// after: the callee can't claim it kept it.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn zero(&mut self) {
        self.n = 0;
    }

    fn keep(&mut self) ensures self.get() == old(self.get()) {
        self.zero();
    }
}

fn main() {}
// error: can't prove `self.get() == old(self.get())` for what `keep` ensures
