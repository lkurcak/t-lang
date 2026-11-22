// Nor can such a field be borrowed `&mut` but straight to a call: where
// the bound holds, nothing would prove the refinement again.
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

fn hold<I: Iterator>(c: &mut Counted<I>) -> Option<I::Item> with I::e {
    let r = &mut c.iter;
    r.next()
}

fn main() {}
// error: can't borrow `c.iter` as `&mut`: the refinement of field `next` of `Counted` names it
