// The bound may hold of a type that's only partly known, `Skip<J>`, as
// it does when `J` is bounded: the body is walked with that.
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

fn late<J: Iterator>(iter: core::iter::Skip<J>) -> Counted<core::iter::Skip<J>>
where
    J::Item: Discard,
{ Counted { iter, next: greatest::<usize>() } }

fn main() {}
// error: where `Skip<J>: BoundedIterator`, which the refinement of field `next` of `Counted` hangs on
