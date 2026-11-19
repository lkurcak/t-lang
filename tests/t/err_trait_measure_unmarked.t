// An impl's method of a trait's measure is a measure too, and says so.
trait Sized {
    #[measure]
    fn size(&self) -> usize;
}

#[derive(Discard)]
struct A { n: usize }

impl Sized for A {
    fn size(&self) -> usize { self.n }
}

fn main() {}
// error: `size` is a measure in trait `Sized`, so `A`'s must be one too
