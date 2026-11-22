// A field the refinement names, lent to a call, is proved again after
// it, where the bound holds: `put` may leave an iterator with more.
use core::num::greatest;

/// Counts down: it has `n` values left.
#[derive(Discard)]
struct Down { n: usize }

impl Iterator for Down {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

impl BoundedIterator for Down {
    #[measure]
    fn most(&self) -> usize { self.n }
}

/// Each value of `iter` with its index, which can't overflow where `iter`
/// says how many it has at most.
#[derive(Discard where I: Discard)]
struct Counted<I> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() where I: BoundedIterator },
}

fn put<I>(at: &mut I, it: I) -> I { core::mem::replace(at, it) }

fn refill<I: Iterator>(c: &mut Counted<I>, more: I) -> I { put(&mut c.iter, more) }

fn main() {}
// error: for field `next` of `Counted`, which names `iter`
