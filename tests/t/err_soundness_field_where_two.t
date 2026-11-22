// A body with the struct at two types is walked under each one's bound:
// the first is kept, and the second isn't.
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

fn both<A: Iterator, B: Iterator>(a: A, b: B) -> (Counted<A>, Counted<B>) {
    (Counted { iter: a, next: 0 }, Counted { iter: b, next: greatest::<usize>() })
}

fn main() {}
// error: where `B: BoundedIterator`, which the refinement of field `next` of `Counted` hangs on
