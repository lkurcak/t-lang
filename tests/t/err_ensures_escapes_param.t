// A borrow stashed in another argument outlives the call too.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

struct Hold<'a> { c: Option<&'a mut Counter> }

fn stash<'a>(c: &'a mut Counter, h: &mut Hold<'a>) ensures c.get() == 0 {
    c.n = 0;
    h.c = Option::Some(c);
}

fn main() {}
// error: `ensures` can't name `c` at return: the borrow it's lent can outlive the call, as the lifetime of `c` is in the signature elsewhere; write `old(..)`
