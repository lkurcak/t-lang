// The borrow of `self` is also in another parameter: the method could
// keep a `&mut` to anything in `self` there, and the caller write through
// it after, so it can't promise what's left.
use alloc::list::List;

struct Bag<T> {
    items: List<T>,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    fn slot<'a>(&'a mut self, keep: &mut Option<&'a mut List<T>>) -> &'a mut List<T>
    ensures
        self.size() == old(self.size()),
    {
        let _ = keep;
        &mut self.items
    }
}

fn main() {}
// error: `ensures` can't name `self` at return
