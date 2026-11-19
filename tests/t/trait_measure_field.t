// An impl's measure may call its trait's measures of a field, a smaller
// value: a wrapper's size is what it wraps (docs/86-trait-contracts.md).
trait Sized {
    #[measure]
    fn size(&self) -> usize;
}

#[derive(Discard)]
struct Leaf { n: usize }

impl Sized for Leaf {
    #[measure]
    fn size(&self) -> usize { self.n }
}

#[derive(Discard)]
struct Wrap<X> { inner: X }

impl<X: Sized + Discard> Sized for Wrap<X> {
    #[measure]
    fn size(&self) -> usize { self.inner.size() }
}

fn under<S: Sized>(s: &S, a: &[u8]) -> u8 {
    if s.size() < a.len() { a[s.size()] } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = Wrap { inner: Wrap { inner: Leaf { n: 2 } } };
    // Known all the way down: 2.
    let k: { k: usize | k == 2 } = w.size();
    out.line(&k)?;                       // expect: 2
    out.line(&under(&w, &[5, 6, 7]))?;   // expect: 7
    Ok(())
}
