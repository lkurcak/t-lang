// A law of a method of a trait that isn't a supertrait holds where `Self`
// is both, as its `where` says: checked for each pair of impls of the two
// at one type (docs/88-iterator-laws.md).
trait Size {
    #[measure]
    fn size(&self) -> usize;
}

trait Shrink {
    fn shrink(&mut self) -> bool;
}

trait Exact: Size {
    law fn shrink(&mut self) -> { b: bool | b == (old(self.size()) > 0) }
    where
        Self: Shrink,
    ensures
        self.size() + 1 == old(self.size()) || (self.size() == 0 && old(self.size()) == 0);
}

#[derive(Discard)]
struct Pile { n: usize }

impl Size for Pile {
    #[measure]
    fn size(&self) -> usize { self.n }
}

impl Exact for Pile {}

impl Shrink for Pile {
    fn shrink(&mut self) -> bool {
        if self.n > 0 {
            self.n = self.n - 1;
            true
        } else {
            false
        }
    }
}

fn empty<T: Exact + Shrink>(t: &mut T) -> { n: usize | n == old(t.size()) } {
    let start = t.size();
    let mut n: usize = 0;
    while t.shrink() invariant n + t.size() == start {
        n = n + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&empty(&mut Pile { n: 4 }))?; // expect: 4
    Ok(())
}
