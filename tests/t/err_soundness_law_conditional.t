// A law of a method that isn't a supertrait's holds where the type is
// both: so a type implementing both is checked against it, wherever each
// impl is (docs/88-iterator-laws.md).
#[derive(Discard)]
struct Both { n: usize }

impl Iterator for Both {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n > 0 {
            self.n = self.n - 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl BoundedIterator for Both {
    #[measure]
    fn most(&self) -> usize { self.n }
}

impl ExactSizeIterator for Both {}

impl DoubleEndedIterator for Both {
    // Gives one, and takes none off.
    fn next_back(&mut self) -> Option<usize> {
        if self.n > 0 { Some(0) } else { None }
    }
}

fn main() {}
// error: can't prove `!r.is_some() || self.most() < old(self.most())` for the result of `next_back`
// error: can't prove `self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0)` for what `next_back` ensures
