// An `impl` result may hold anything of the signature's: what it can
// write isn't known, so nothing is promised past the call.
use alloc::list::List;

struct Bag<T> {
    items: List<T>,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    fn each<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    ensures
        self.size() == old(self.size()),
    {
        self.items.iter_mut()
    }
}

fn main() {}
// error: `ensures` can't name `self` at return
