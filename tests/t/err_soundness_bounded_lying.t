// A `BoundedIterator`'s `most` takes at least one off for each value its
// `next` gives: each impl is checked to, or `count` of it could overflow
// (docs/92-where-a-bound-holds.md).
struct Up { n: usize }

impl Iterator for Up {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n < 9 {
            self.n = self.n + 1;
        }
        Some(self.n)
    }
}

impl BoundedIterator for Up {
    #[measure]
    fn most(&self) -> usize { self.n }
}

fn main() {}
// error: can't prove `!r.is_some() || self.most() < old(self.most())` for the result of `next`
