// An inherent measure of a generic type may call its type parameter's
// trait's measure: at a known type, it's that type's impl's, in the walk
// and in the interpreter, which finds the copy at the type
// (docs/86-trait-contracts.md).
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
struct W<I> { it: I, extra: { e: usize | e < 10 } }

impl<I: Sized> W<I> {
    #[measure]
    fn total(&self) -> usize { self.it.size() / 2 + self.extra }
}

fn sum<I: Sized>(w: &W<I>) -> { t: usize | t == w.total() } { w.total() }

fn pick(a: &[u8], w: &W<Leaf>) -> u8 {
    if w.total() < a.len() { a[w.total()] } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = W { it: Leaf { n: 1 }, extra: 1 };
    let t: { t: usize | t == 1 } = sum(&w);
    out.line(&t)?;                     // expect: 1
    out.line(&pick(&[4, 5, 6], &w))?;  // expect: 5
    Ok(())
}
