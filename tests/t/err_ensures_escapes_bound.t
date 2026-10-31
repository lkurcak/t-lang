// A bound saying the lifetime outlives another lets the borrow be kept
// where that one is.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

fn stash<'b, 'a: 'b>(c: &'a mut Counter, h: &mut Option<&'b mut Counter>) ensures c.get() == 0 {
    c.n = 0;
    *h = Option::Some(c);
}

fn main() {}
// error: `ensures` can't name `c` at return: the borrow it's lent can outlive the call, as the lifetime of `c` is in the signature elsewhere; write `old(..)`
