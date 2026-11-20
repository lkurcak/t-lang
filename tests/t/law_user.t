// A trait of one's own can state a law of its supertrait's method: what
// every type implementing it keeps, checked against each impl's body, and
// learned by every call where the type implements it
// (docs/88-iterator-laws.md).
trait Counted: Iterator {
    #[measure]
    fn left(&self) -> usize;

    law fn next(&mut self) -> { r: Option<Self::Item> | r.is_some() == (old(self.left()) > 0) }
    with Self::e
    ensures
        self.left() + 1 == old(self.left()) || (self.left() == 0 && old(self.left()) == 0);
}

#[derive(Discard)]
struct Down { n: usize }

impl Iterator for Down {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n > 0 {
            self.n = self.n - 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl Counted for Down {
    #[measure]
    fn left(&self) -> usize { self.n }
}

fn drain<I: Counted>(it: &mut I) -> { n: usize | n == old(it.left()) }
with I::e
where
    I::Item: Discard,
{
    let start = it.left();
    let mut n: usize = 0;
    while let Some(_x) = it.next() invariant n + it.left() == start {
        n = n + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut d = Down { n: 3 };
    out.line(&drain(&mut d))?; // expect: 3
    // At the known type too.
    let mut e = Down { n: 1 };
    let a = e.next();
    let b = e.next();
    let ok: { v: bool | v } = a.is_some() && b.is_none();
    out.line(&ok)?; // expect: true
    Ok(())
}
