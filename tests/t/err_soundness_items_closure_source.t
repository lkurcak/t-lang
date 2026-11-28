// An adapter whose closure gives values of its iterator's `Item` can give
// those, which the iterator never gave.
#[derive(Discard)]
struct Fill<I, F> { iter: I, f: F }

impl<I: Iterator, F: FnMut() -> I::Item> Iterator for Fill<I, F> where I::Item: Discard {
    type Item = I::Item;
    effect e = {I::e, F::e};

    fn next(&mut self) -> Option<I::Item> with {I::e, F::e} {
        match self.iter.next() {
            Some(_x) => Some((self.f)()),
            None => None,
        }
    }
}

fn fill<I: Iterator, F: FnMut() -> I::Item>(iter: I, f: F) -> Fill<I, F> where I::Item: Discard {
    Fill { iter, f }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in fill(0..8, || 9) {
        let _ = a[r];
    }
}
// error: can't prove
