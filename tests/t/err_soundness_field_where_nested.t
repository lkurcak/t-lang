// A struct in another's field is at a place like any other.
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

#[derive(Discard where I: Discard)]
struct Outer<I> { inner: Counted<I> }

fn jump<I: Iterator>(o: &mut Outer<I>) {
    o.inner.next = greatest::<usize>();
}

fn main() {}
// error: can't prove `n + iter.most() <= greatest::<usize>()` for field `next` of `Counted`
