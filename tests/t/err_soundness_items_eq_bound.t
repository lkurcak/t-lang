// An adapter that knows what its iterator's `Item` is can make values of
// it: here, `9`, which `0..8` never gives.
#[derive(Discard)]
struct Nine<I> { iter: I }

impl<I: Iterator<Item = usize>> Iterator for Nine<I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Some(_x) => Some(9),
            None => None,
        }
    }
}

fn nine<I: Iterator<Item = usize>>(iter: I) -> Nine<I> { Nine { iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in nine(0..8) {
        let _ = a[r];
    }
}
// error: can't prove
