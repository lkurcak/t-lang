// An impl's measure can't come back to itself, of the same value, through
// the trait's defaults: `full` of `self` would never end
// (docs/86-trait-contracts.md).
trait Bin {
    #[measure]
    fn full(&self) -> bool;

    #[measure]
    fn empty(&self) -> bool { !self.full() }
}

#[derive(Discard)]
struct S { n: usize }

impl Bin for S {
    #[measure]
    fn full(&self) -> bool { !self.empty() }
}

fn main() {}
// error: measure `full` of `S` calls itself on the same value, through `empty`
