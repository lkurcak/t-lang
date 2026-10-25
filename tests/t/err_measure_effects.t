// What a measure gives must be all it does: it can't panic, or allocate.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len(&self) -> usize with panic { self.n }
}

fn main() {}
// error: a measure can't have effects
