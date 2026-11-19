// An impl's method is a measure only if its trait's is.
trait Sized {
    fn size(&self) -> usize;
}

#[derive(Discard)]
struct A { n: usize }

impl Sized for A {
    #[measure]
    fn size(&self) -> usize { self.n }
}

fn main() {}
// error: `size` isn't a measure in trait `Sized`
