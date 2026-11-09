// An `impl` result may hold the borrow of `self` though no lifetime in its
// type says so: this closure pushes onto the list each time it's called.
use alloc::list::List;

#[derive(Discard)]
struct Bag {
    items: List<u8>,
}

impl Bag {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    fn pusher(&mut self) -> impl FnMut() with alloc ensures self.size() == old(self.size()) {
        move || self.items.push(0)
    }
}

fn main() with alloc {
    let mut b = Bag { items: List::new() };
    if b.size() == 0 {
        let mut f = b.pusher();
        f();
        let n: { n: usize | n == 0 } = b.size();
        let _ = n;
    }
}
// error: `ensures` can't name `self` at return
