// An impl's measure is the trait's, at its type: what a generic function
// ensures of it holds of the impl's body (docs/86-trait-contracts.md).
trait Count {
    #[measure]
    fn count(&self) -> u8;

    fn bump(&mut self) ensures self.count() == old(self.count()) + 1;
}

#[derive(Discard)]
struct C { n: u8, m: u8 }

impl Count for C {
    #[measure]
    fn count(&self) -> u8 { self.n }

    // Bumps the wrong one.
    fn bump(&mut self) {
        if self.m < 200 {
            self.m = self.m + 1;
        }
    }
}

fn main() {}
// error: can't prove `self.count() == old(self.count()) + 1` for what `bump` ensures
