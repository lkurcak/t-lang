// A trait's default method that makes one of `Self` is walked with the
// bound on `Self`, for the impls whose types meet it.
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

trait Number: Iterator {
    fn numbered(self) -> Counted<Self> { Counted { iter: self, next: greatest::<usize>() } }
}

impl Number for Down {}

fn main() {}
// error: where `Self: BoundedIterator`, which the refinement of field `next` of `Counted` hangs on
