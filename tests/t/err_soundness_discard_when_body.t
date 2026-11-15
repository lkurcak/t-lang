// An impl's `discard` knows only what its refinement says: here not that
// the list inside is empty, so it can't drop it.
use alloc::list::List;

struct Fd { n: i64 }

struct Bag { items: List<Fd>, count: i64 }

impl Bag {
    #[measure]
    fn count(&self) -> i64 { self.count }
}

impl Discard for { b: Bag | b.count() == 0 } {
    fn discard(self) {
        let Bag { items, count: _ } = self;
    }
}

fn main() with alloc {
    let mut b = Bag { items: List::new(), count: 0 };
    b.items.push(Fd { n: 1 });
}
// error: `items` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
