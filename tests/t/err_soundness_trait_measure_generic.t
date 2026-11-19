// A trait's measure is, in generic code, what's been learned of it: the
// trait says nothing of its value here (docs/86-trait-contracts.md).
trait Sized {
    #[measure]
    fn size(&self) -> usize;
}

fn first<S: Sized>(s: &S, a: &[u8]) -> u8 {
    a[s.size()]
}

fn main() {}
// error: can't prove `i < self.len()`
