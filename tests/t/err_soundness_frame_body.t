// The method itself must keep what it promises up to its return: this one
// pushes before giving the slot.
use alloc::list::List;

struct Bag<T> {
    items: List<T>,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }
}

impl<T: Discard> Bag<T> {
    fn grow_slot(&mut self, v: T) -> &mut T with alloc ensures self.size() == old(self.size()) {
        self.items.push(v);
        &mut self.items[0]
    }
}

fn main() {}
// error: can't prove `self.size() == old(self.size())`
