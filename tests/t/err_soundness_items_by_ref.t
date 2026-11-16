// An adapter that holds its iterator behind a `&mut` doesn't keep it: what
// it points to can be written through the field, which no call of the
// adapter's protocol does, and then it's another iterator.
use core::iter::Rev;
use core::ops::Range;

#[derive(Discard)]
struct ByRef<'a, I> { it: &'a mut I }

impl<'a, I: Iterator> Iterator for ByRef<'a, I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.it.next() }
}

fn by_ref<'a, I: Iterator>(it: &'a mut I) -> ByRef<'a, I> { ByRef { it } }

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut rv: Rev<Range<usize>> = (0..8).rev();
    let mut b = by_ref(&mut rv);
    *b.it = (0..100).rev();
    while let Some(x) = b.next() {
        let _ = a[x];
    }
}
// error: can't prove
