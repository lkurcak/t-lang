// Or of the type behind an `impl Trait`, which a copy of the body has.
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

fn downs() -> impl Iterator<Item = usize, e = {}> + Discard { Down { n: 3 } }

fn main() with {} {
    let c = Counted { iter: downs(), next: greatest::<usize>() };
    let _n = c.next;
}

// error: where `impl Iterator<Item = usize, e = {}> + Discard: BoundedIterator`, which the refinement of field `next` of `Counted` hangs on
