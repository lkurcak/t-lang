// An adapter whose closure is lent its iterator `&mut` can be given
// another: what it gives isn't only what `0..8` does.
#[derive(Discard)]
struct Redo<I, F> { iter: I, f: F }

impl<I: Iterator, F: FnMut(&mut I)> Iterator for Redo<I, F> {
    type Item = I::Item;
    effect e = {I::e, F::e};

    fn next(&mut self) -> Option<I::Item> with {I::e, F::e} {
        (self.f)(&mut self.iter);
        self.iter.next()
    }
}

fn redo<I: Iterator, F: FnMut(&mut I)>(iter: I, f: F) -> Redo<I, F> {
    Redo { iter, f }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in redo(0..8, |it: &mut core::ops::Range<usize>| *it = 100..101) {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()`
