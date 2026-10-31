// Every way out of the function must leave what `ensures` says: an early
// `return` too.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn clear(&mut self, early: bool) ensures self.get() == 0 {
        if early {
            return;
        }
        self.n = 0;
    }
}

fn main() {}
// error: can't prove `self.get() == 0` for what `clear` ensures
