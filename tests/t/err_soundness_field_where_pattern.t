// A pattern's `&mut` binding of the refined field is a `&mut` of it:
// what's written through it must have the refinement where the bound
// holds.
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

fn reset<I: Iterator>(c: &mut Counted<I>) {
    match c {
        Counted { next, .. } => *next = greatest::<usize>(),
    }
}

fn main() {}
// error: can't bind `next` by `&mut`: field `next` of `Counted` has a refinement
