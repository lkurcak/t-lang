// A measure is one function of the value: it has no parameters of its own.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len<T>(&self) -> usize { self.n }
}

fn main() {}
// error: a measure has no type parameters of its own
