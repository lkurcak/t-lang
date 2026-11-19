// An impl of a trait with a law of `next` must keep it: its `Iterator`
// impl's `next` is checked to, or generic code counting on it would be
// wrong of this type (docs/88-iterator-laws.md).
#[derive(Discard)]
struct Stuck { n: usize }

impl Iterator for Stuck {
    type Item = usize;
    effect e = {};

    // Gives a value, but `len` stays.
    fn next(&mut self) -> Option<usize> {
        if self.n > 0 { Some(self.n) } else { None }
    }
}

impl ExactSizeIterator for Stuck {
    #[measure]
    fn len(&self) -> usize { self.n }
}

fn main() {}
// error: can't prove `self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0)` for what `next` ensures
