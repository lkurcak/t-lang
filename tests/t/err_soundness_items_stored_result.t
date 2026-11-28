// An adapter that holds a value of its closure's result type in a field
// can give that value, which its closure never gave: so what it gives
// isn't what the closure's summary says.
#[derive(Discard where I: Discard, F: Discard, U: Discard)]
struct Then<I, F, U> { iter: I, f: F, last: Option<U> }

impl<I: Iterator, F: FnMut(I::Item) -> U, U> Iterator for Then<I, F, U> {
    type Item = U;
    effect e = {I::e, F::e};

    fn next(&mut self) -> Option<U> with {I::e, F::e} {
        match self.last.take() {
            Some(u) => Some(u),
            None => match self.iter.next() {
                Some(x) => Some((self.f)(x)),
                None => None,
            },
        }
    }
}

fn then<I: Iterator, F: FnMut(I::Item) -> U, U>(iter: I, f: F, last: U) -> Then<I, F, U> {
    Then { iter, f, last: Some(last) }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in then(0..8, |x| x / 2, 9) {
        let _ = a[r];
    }
}
// error: can't prove
