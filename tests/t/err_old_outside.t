// `old(..)` is the value at the call, which only a postcondition has a
// later one to tell apart from.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

fn f(c: &mut Counter, k: { k: usize | k < old(c.get()) }) {}

fn main() {}
// error: `old(..)` can only be in a postcondition: `ensures`, or a result's refinement
