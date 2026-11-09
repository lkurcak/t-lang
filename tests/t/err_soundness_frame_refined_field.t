// A field refined by a measure the method doesn't promise to keep can't be
// lent to it: its refinement couldn't be proved again after.
use alloc::list::List;

struct Bag<T> {
    items: List<T>,
    extra: usize,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    #[measure]
    fn more(&self) -> usize { self.extra }

    fn slot(&mut self, i: { i: usize | i < self.size() }) -> &mut T
    ensures
        self.size() == old(self.size()),
    {
        &mut self.items[i]
    }
}

struct Holder {
    bag: { b: Bag<u8> | b.more() == 3 && b.size() > 0 },
}

impl Holder {
    fn first(&mut self) { *self.bag.slot(0) = 1; }
}

fn main() {}
// error: can't prove `b.more() == 3 && b.size() > 0`
