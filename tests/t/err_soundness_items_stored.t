// An adapter that holds a value of its iterator's `Item` in a field can
// give that value, which its iterator never gave: so what it gives is
// what's declared of `Item`, nothing more.
struct Pre<I: Iterator> { first: Option<I::Item>, iter: I }

impl<I: Iterator> Iterator for Pre<I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.first.take() {
            Some(x) => Some(x),
            None => self.iter.next(),
        }
    }
}

fn pre<I: Iterator>(iter: I, first: I::Item) -> Pre<I> { Pre { first: Some(first), iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = pre(0..8, 9);
    while let Some(r) = it.next() {
        let _ = a[r];
    }
    // At its own type, what's in it can be thrown away.
    let Pre { first: _, iter: _ } = it;
}
// error: can't prove
