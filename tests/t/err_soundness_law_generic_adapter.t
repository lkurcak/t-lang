// A generic adapter's `next` keeps the law under its impl's bounds: here
// it takes two of its iterator's values for each it gives, so its `len`
// can't be its iterator's (docs/88-iterator-laws.md).
#[derive(Discard where I: Discard)]
struct Pairs<I> { iter: I }

impl<I: Iterator> Iterator for Pairs<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        let _first = self.iter.next();
        self.iter.next()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Pairs<I> where I::Item: Discard {
    #[measure]
    fn len(&self) -> usize { self.iter.len() }
}

fn main() {}
// error: can't prove `r.is_some() == (old(self.len()) > 0)` for the result of `next`
// error: can't prove `self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0)` for what `next` ensures
