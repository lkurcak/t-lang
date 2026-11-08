// An impl's method of a trait may ensure more than the trait, but it must
// prove it, as any function must.
trait Bump {
    fn bump(&mut self);
}

struct C { n: u8 }

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }
}

impl Bump for C {
    fn bump(&mut self) ensures self.n() == old(self.n()) {
        if self.n < 200 {
            self.n = self.n + 1;
        }
    }
}

fn main() {}
// error: can't prove `self.n() == old(self.n())` for what `bump` ensures
