// A method that says nothing of what it leaves says nothing: the caller
// knows nothing of the list after, though the result writes only `T`s.
use alloc::list::List;

#[derive(Discard where T: Discard)]
struct Bag<T> {
    items: List<T>,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    fn slot(&mut self, i: { i: usize | i < self.size() }) -> &mut T { &mut self.items[i] }
}

fn main() with alloc {
    let mut b = Bag { items: List::filled(2, 0u8) };
    if b.size() == 2 {
        *b.slot(0) = 1;
        *b.slot(1) = 1;
    }
}
// error: can't prove `i < self.size()` for argument `i` of `slot`
