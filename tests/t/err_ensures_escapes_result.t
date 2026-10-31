// What's behind a `&mut` the result can hold isn't done changing when the
// function returns: a postcondition can only say what it was, `old(..)`.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn slot(&mut self) -> &mut usize ensures self.get() == old(self.get()) { &mut self.n }
}

fn main() {}
// error: `ensures` can't name `self` at return: the borrow it's lent can outlive the call, as the lifetime of `self` is in the signature elsewhere; write `old(..)`
