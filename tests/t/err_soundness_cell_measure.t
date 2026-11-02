// A measure can't read inside a cell: what it reads must not change
// through a `&` (lemma M1 of docs/21-measures.md).
use core::cell::Cell;

struct Counter { n: Cell<i64> }

impl Counter {
    #[measure]
    fn count(&self) -> i64 { self.n.get() }
}

fn main() {}
// error: isn't a measure
