// An adapter whose impl can make values of its items (here by an `Int`
// bound on `I::Item`, whose literals make them) says nothing of them,
// even inside another adapter: `skip`'s items are `Made`'s, which aren't
// the list's.
use alloc::list::List;

#[derive(Discard where I: Discard)]
struct Made<I> { i: I }

impl<I: Iterator> Iterator for Made<I> where I::Item: Int {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.i.next() {
            Some(_) => Some(99),
            None => None,
        }
    }
}

fn main() with {alloc, panic} {
    let a: [u8; 5] = [0, 1, 2, 3, 4];
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let z = Made { i: l.as_slice().iter().copied() };
    for x in z.skip(0) {
        let _ = a[x];
    }
}
// error: can't prove `i < self.len()`
