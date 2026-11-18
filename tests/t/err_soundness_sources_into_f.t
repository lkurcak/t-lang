// An adapter's own module can give its closure back (`into_f`): so it's
// not sealed, and the closure isn't walked knowing what it's given.
fn need8(_r: { r: usize | r < 8 }) {}

#[derive(Discard)]
struct Each<I, F> { iter: I, f: F }

impl<I: Iterator, F: FnMut(I::Item) -> bool> Iterator for Each<I, F> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I, F> Each<I, F> {
    fn into_f(self) -> F where I: Discard { self.f }
}

fn each<I: Iterator, F: FnMut(I::Item) -> bool>(iter: I, f: F) -> Each<I, F> where I::Item: Discard { Each { iter, f } }

fn main() {
    let e = each(0..8, |x: usize| { need8(x); true });
    let mut f = e.into_f();
    let _ = f(100);
}
// error: can't prove `r < 8`
