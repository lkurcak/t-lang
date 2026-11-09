// The borrow of `self` is also in another parameter: `slot` keeps a
// `&mut` to the list there, which the caller pushes through after, so it
// can't promise the size stays, though its result writes only a `T`.
use alloc::list::List;

#[derive(Discard where T: Discard)]
struct Bag<T> {
    items: List<T>,
    first: T,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    fn slot<'a>(&'a mut self, keep: &mut Option<&'a mut List<T>>) -> &'a mut T
    ensures
        self.size() == old(self.size()),
    {
        *keep = Some(&mut self.items);
        &mut self.first
    }
}

fn main() with alloc {
    let mut b = Bag { items: List::new(), first: 0u8 };
    if b.size() == 0 {
        let mut keep: Option<&mut List<u8>> = None;
        let _ = b.slot(&mut keep);
        if let Some(l) = keep {
            l.push(1);
        }
        let n: { n: usize | n == 0 } = b.size();
        let _ = n;
    }
}
// error: `ensures` can't name `self` at return
