// Measures defined by each other would say nothing: `a() == b() + 1` and
// `b() == a() + 1` can't both be.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn a(&self) -> bool { !self.b() }

    #[measure]
    fn b(&self) -> bool { !self.a() }
}

fn main() {}
// error: measure `b` is defined by itself, through `a`
