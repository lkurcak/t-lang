// A measure's body is a term of the logic: fields, measures, literals,
// arithmetic and comparisons. An `if` isn't one.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len(&self) -> usize { if self.n > 3 { 3 } else { self.n } }
}

fn main() {}
// error: a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*` by a constant, comparisons, `&&`, `||` and `!`
