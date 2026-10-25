// A result's refinement is about what its arguments were at the call:
// what's behind a `&mut` may have changed since, as here.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len(&self) -> usize { self.n }
}

fn grow(w: &mut W) -> { k: usize | k == w.len() } {
    w.n = 0;
    w.len()
}

fn main() {}
// error: can't prove `k == w.len()` for the result of `grow`
