// What `ensures` says is proved of what the function leaves.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn reset(&mut self) ensures self.get() == old(self.get()) {
        self.n = 0;
    }
}

fn main() {}
// error: can't prove `self.get() == old(self.get())` for what `reset` ensures
