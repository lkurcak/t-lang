// A measure's body is all it does, so its arithmetic must be proved not to
// overflow: the logic's `+` doesn't.
#[derive(Copy, Discard)]
struct W { a: usize, b: usize }

impl W {
    #[measure]
    fn total(&self) -> usize { self.a + self.b }
}

fn main() {}
// error: `+` may panic, but `total` doesn't say `with panic`
