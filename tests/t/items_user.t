// An iterator of the program's own gets what every value it gives has from
// its signatures, as a range does, and an adapter of its own passes on what
// its iterator's give (docs/82-item-invariants.md).

/// Counts up from `at` to below `hi`; `hi` never grows, `at` never shrinks.
#[derive(Discard)]
struct Up { at: u32, hi: u32 }

impl Up {
    fn new(at: u32, hi: u32) -> { u: Up | u.at() == at && u.hi() == hi } { Up { at, hi } }

    #[measure]
    fn at(&self) -> u32 { self.at }

    #[measure]
    fn hi(&self) -> u32 { self.hi }
}

impl Iterator for Up {
    type Item = u32;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u32 | v >= old(self.at()) && v < old(self.hi()) }>
    ensures
        self.hi() == old(self.hi()),
        self.at() >= old(self.at()),
    {
        if self.at < self.hi {
            let v = self.at;
            self.at = self.at + 1;
            Some(v)
        } else {
            None
        }
    }
}

/// Every other value of `iter`.
#[derive(Discard where I: Discard)]
struct Halve<I> { iter: I }

impl<I: Iterator> Iterator for Halve<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Some(_) => self.iter.next(),
            None => None,
        }
    }
}

fn halve<I: Iterator>(iter: I) -> Halve<I> where I::Item: Discard { Halve { iter } }

/// Skips one, through a `&mut`: what's left still gives only what it did.
fn drop_one<I: Iterator>(it: &mut I) with I::e where I::Item: Discard {
    let _ = it.next();
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 6] = [10, 11, 12, 13, 14, 15];
    for v in Up::new(2, 6) {
        out.line(&a[v as usize])?;
    }
    // expect: 12
    // expect: 13
    // expect: 14
    // expect: 15
    let mut h = halve(Up::new(0, 6));
    drop_one(&mut h);
    while let Some(v) = h.next() {
        out.line(&a[v as usize])?;
    }
    // expect: 13
    // expect: 15
    for v in halve((0..6).rev()) {
        out.line(&a[v])?;
    }
    // expect: 14
    // expect: 12
    // expect: 10
    Ok(())
}
