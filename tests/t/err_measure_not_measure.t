// A refinement can call only measures: what else a method does, the logic
// can't know.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    fn len(&self) -> usize { self.n }
}

fn at(w: &W, i: { i: usize | i < w.len() }) -> usize { i }

fn main() {}
// error: `len` isn't a measure: a refinement can only call methods marked `#[measure]`
