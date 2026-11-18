// An adapter's `DoubleEndedIterator` impl has bounds of its own: this one
// knows its items are `usize`s and makes one.
#[derive(Discard)]
struct Nine<I> { iter: I }

impl<I: Iterator> Iterator for Nine<I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I: DoubleEndedIterator<Item = usize>> DoubleEndedIterator for Nine<I> {
    fn next_back(&mut self) -> Option<usize> with I::e {
        match self.iter.next_back() {
            Some(_x) => Some(9),
            None => None,
        }
    }
}

fn nine<I: Iterator>(iter: I) -> Nine<I> { Nine { iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in nine(0..8).rev() {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()`
