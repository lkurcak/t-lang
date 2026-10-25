// A measure can't change what it measures, so it takes `&self`.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len(&mut self) -> usize { self.n }
}

fn main() {}
// error: a measure takes only `&self`
