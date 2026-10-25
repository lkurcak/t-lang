// A measure is of the value alone: it takes nothing but `&self`.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn plus(&self, k: usize) -> usize { self.n }
}

fn main() {}
// error: a measure takes only `&self`
