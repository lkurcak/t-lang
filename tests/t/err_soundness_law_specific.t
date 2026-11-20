// An impl of a law's trait for fewer types than its supertrait's impl
// would need that impl's body checked at those types, which isn't done:
// refused, not trusted (docs/88-iterator-laws.md).
#[derive(Discard where T: Discard)]
struct Once<T> { v: Option<T> }

impl<T> Iterator for Once<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<T> {
        let mut v = None;
        core::mem::swap(&mut v, &mut self.v);
        v
    }
}

impl ExactSizeIterator for Once<u8> {
    #[measure]
    fn len(&self) -> usize { 1 }
}

fn main() {}
// error: can't check that `Once<u8>` keeps `ExactSizeIterator`'s law of `next`: its impl of `Iterator` is for more types than this one
