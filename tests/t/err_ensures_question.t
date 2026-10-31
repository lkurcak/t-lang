// `?` returns too, so what `ensures` says must hold there.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn clear(&mut self, r: Result<(), bool>) -> Result<(), bool> ensures self.get() == 0 {
        r?;
        self.n = 0;
        Result::Ok(())
    }
}

fn main() {}
// error: can't prove `self.get() == 0` for what `clear` ensures
