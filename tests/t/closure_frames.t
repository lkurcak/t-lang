// What a closure's calls keep of what it captures by `&mut`: each change
// its body makes to the place is a call that keeps a measure, so its loan
// keeps it, as a loop's frame does (docs/69-loans.md).
use alloc::list::List;

#[derive(Discard)]
struct M {
    buckets: List<List<u64>>,
    len: usize,
}

impl M {
    #[measure]
    fn len(&self) -> usize { self.len }

    /// Puts `e` in a bucket of its own; `len` is the caller's.
    fn put(&mut self, e: u64) with alloc ensures self.len() == old(self.len()) {
        let mut b = List::new();
        b.push(e);
        self.buckets.push(b);
    }

    /// As a hash map grows: the closures capture `self`, and the only
    /// change they make to it is `put`, which keeps `len()`.
    fn grow(&mut self) with alloc ensures self.len() == old(self.len()) {
        let old = core::mem::replace(&mut self.buckets, List::new());
        old.free_with(|b: List<u64>| b.free_with(|e: u64| self.put(e)));
    }

    /// A closure called here, twice.
    fn twice(&mut self) with alloc ensures self.len() == old(self.len()) {
        let mut p = |e: u64| self.put(e);
        p(1);
        p(2);
    }
}

#[derive(Copy, Discard)]
struct C {
    n: u8,
    m: u8,
}

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = self.m / 2; }
}

/// A loop that makes a closure keeping `n()` each time round: its head
/// knows `n()` too.
fn rounds(c: &mut C) -> u8 with {} {
    c.n = 4;
    let mut i: u8 = 0;
    while i < 3 {
        let mut t = || c.touch();
        t();
        let _q = 8 / c.n;
        i = i + 1;
    }
    c.n
}

/// A closure that moves a `&mut` in borrows what it points to: the same.
fn moved(c: &mut C) -> u8 with {} {
    c.n = 2;
    let mut t = move || c.touch();
    t();
    8 / c.n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut m = M { buckets: List::new(), len: 0 };
    m.put(7);
    m.len = 1;
    m.grow();
    m.twice();
    let l: { l: usize | l == 1 } = m.len();
    out.line(&l)?; // expect: 1
    out.line(&m.buckets.len())?; // expect: 3
    let mut c = C { n: 0, m: 9 };
    out.line(&rounds(&mut c))?; // expect: 4
    out.line(&moved(&mut c))?; // expect: 4
    Ok(())
}
