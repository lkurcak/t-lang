// A measure's body can use measures, not other methods.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    fn raw(&self) -> usize { self.n }

    #[measure]
    fn len(&self) -> usize { self.raw() }
}

fn main() {}
// error: `raw` isn't a measure: a refinement can only call methods marked `#[measure]`
