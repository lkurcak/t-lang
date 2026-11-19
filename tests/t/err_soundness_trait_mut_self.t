// A trait's method's promise of its `self`'s measure is of `self` as
// passed: an impl's can't take it `mut self`.
trait Tr {
    #[measure]
    fn size(&self) -> usize;
    fn consume(self) -> { n: usize | n <= self.size() };
}

#[derive(Discard)]
struct X { n: usize }

impl Tr for X {
    #[measure]
    fn size(&self) -> usize { self.n }
    fn consume(mut self) -> usize {
        self.n = 100;
        self.n
    }
}

fn g<T: Tr + Discard>(t: { t: T | t.size() < 2 }) -> u8 {
    let a = [7u8; 2];
    a[t.consume()]
}

fn main() with {} {
    let _ = g(X { n: 1 });
}
// error: `self` can't be `mut` in `consume`: what trait `Tr` promises names it, as it was passed; take a `mut` copy by another name, `let mut s = self;`
